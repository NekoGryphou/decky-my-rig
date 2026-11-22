"""Runs `myrig agent` and relays the plugin's calls to it.

The agent owns the saved PCs, polling, pairing and power control; this side
only matches replies to requests and keeps the latest snapshot frame for the
frontend's long-poll.
"""

import asyncio
import contextlib
import json
import subprocess
from pathlib import Path
from typing import Any

from .agent_consts import (
    EXIT_TIMEOUT_SECONDS,
    LINE_LIMIT,
    MAX_RESTARTS,
    REFUSED_EXIT,
)
from .agent_errors import AgentError
from .agent_messages import INVALID_REVISION, UNAVAILABLE
from .agent_types import Frame, Logger


# noinspection PyTypeHints,PyTypeChecker
class AgentClient:
    def __init__(self, binary: Path, store: Path, logger: Logger) -> None:
        self._binary = binary
        self._store = store
        self._logger = logger
        self._process: asyncio.subprocess.Process | None = None
        self._tasks: list[asyncio.Task[None]] = []
        self._pending: dict[int, asyncio.Future[Any]] = {}
        self._next = 0
        self._latest: Frame | None = None
        self._changed = asyncio.Event()
        self._closed = False
        self._stopping = False
        self._restarts = 0
        self._recovery: asyncio.Task[None] | None = None
        self._refusal: str | None = None
        self._last_error: str | None = None

    async def start(self) -> None:
        try:
            self._process = await asyncio.create_subprocess_exec(
                str(self._binary),
                "agent",
                "--store",
                str(self._store),
                stdin=asyncio.subprocess.PIPE,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE,
                limit=LINE_LIMIT,
                creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0),
            )
        except OSError as error:
            self._logger.error(
                "Decky My Rig agent %s cannot start: %s", self._binary.name, error
            )
            self._closed = True
            self._changed.set()
            return
        self._tasks = [
            asyncio.create_task(self._read_replies()),
            asyncio.create_task(self._log_errors()),
        ]

    async def call(self, method: str, params: dict[str, object]) -> Any:
        process = self._process
        if self._closed or process is None or process.stdin is None:
            raise AgentError(self._refusal or UNAVAILABLE)
        self._next += 1
        identifier = self._next
        reply: asyncio.Future[Any] = asyncio.get_running_loop().create_future()
        self._pending[identifier] = reply
        line = json.dumps({"id": identifier, "method": method, "params": params})
        try:
            process.stdin.write(line.encode() + b"\n")
            await process.stdin.drain()
            return await reply
        except (ConnectionError, RuntimeError):
            raise AgentError(UNAVAILABLE) from None
        finally:
            self._pending.pop(identifier, None)

    async def watch(self, after: object) -> Frame:
        """The latest frame unless the caller saw it, otherwise the next one."""
        if type(after) is not int or after < -1:
            raise AgentError(INVALID_REVISION)
        while True:
            latest = self._latest
            if self._closed:
                return self._closed_frame()
            if latest is not None and latest["revision"] != after:
                return latest
            self._changed.clear()
            await self._changed.wait()

    async def close(self) -> None:
        """Ends the agent: EOF on its stdin, then wait for it to exit.

        Never suspends: when Decky stops a plugin, its socket listener spins
        on EOF without yielding, so an await here would never resume.
        """
        if self._stopping:
            return
        self._stopping = True
        self._closed = True
        self._changed.set()
        if self._recovery is not None:
            self._recovery.cancel()
        for task in self._tasks:
            task.cancel()
        self._tasks = []
        if self._process is not None:
            self._end_now(self._process)
        self._fail_pending()

    def _end_now(self, process: asyncio.subprocess.Process) -> None:
        """EOF on the agent's stdin and its exit, without the event loop."""
        if process.stdin is not None:
            # ! Close the pipe ourselves: the transport would close it on a later
            #   loop turn, which might never come.
            pipe = process.stdin.transport.get_extra_info("pipe")
            if pipe is not None:
                pipe.close()
            process.stdin.close()
        transport = getattr(process, "_transport")  # noqa: B009 - asyncio internal
        popen: subprocess.Popen[bytes] = transport.get_extra_info("subprocess")
        try:
            popen.wait(EXIT_TIMEOUT_SECONDS)
        except subprocess.TimeoutExpired:
            self._logger.error("Decky My Rig agent did not exit; stopping it.")
            popen.kill()
            popen.wait()

    async def _stop_process(self) -> int | None:
        process = self._process
        if process is None:
            return None
        if process.stdin is not None:
            process.stdin.close()
        try:
            await asyncio.wait_for(process.wait(), EXIT_TIMEOUT_SECONDS)
        except TimeoutError:
            self._logger.error("Decky My Rig agent did not exit; stopping it.")
            process.kill()
            await process.wait()
        return process.returncode

    async def _cancel_tasks(self) -> None:
        current = asyncio.current_task()
        for task in self._tasks:
            if task is not current:
                task.cancel()
                with contextlib.suppress(asyncio.CancelledError):
                    await task
        self._tasks = []

    async def _recover(self) -> None:
        """After the agent stopped answering: start it again, unless it
        refused to run or keeps ending."""
        code = await self._stop_process()
        readers = [task for task in self._tasks if task is not asyncio.current_task()]
        if readers:
            await asyncio.wait(readers, timeout=EXIT_TIMEOUT_SECONDS)
        await self._cancel_tasks()
        if self._stopping:
            return
        if code == REFUSED_EXIT and self._last_error:
            self._refusal = self._last_error
        elif self._restarts < MAX_RESTARTS:
            self._restarts += 1
            self._logger.error(
                "Decky My Rig agent ended (%s); restarting it (%d of %d).",
                code,
                self._restarts,
                MAX_RESTARTS,
            )
            await self.start()
            return
        self._closed = True
        self._changed.set()

    async def _read_replies(self) -> None:
        process = self._process
        if process is None or process.stdout is None:
            return
        try:
            while line := await process.stdout.readline():
                self._receive(line)
        except (ValueError, ConnectionError):
            self._logger.error("Decky My Rig agent sent an unreadable line.")
        # - The reader stopped, so nothing pending can get an answer anymore.
        self._fail_pending()
        if not self._stopping:
            self._recovery = asyncio.create_task(self._recover())

    def _receive(self, line: bytes) -> None:
        try:
            message = json.loads(line)
        except ValueError:
            self._logger.error("Decky My Rig agent sent invalid JSON.")
            return
        if not isinstance(message, dict):
            self._logger.error("Decky My Rig agent sent a line that is not an object.")
            return
        if message.get("event") == "snapshot":
            frame = message.get("frame")
            if not isinstance(frame, dict) or type(frame.get("revision")) is not int:
                self._logger.error("Decky My Rig agent sent an invalid snapshot.")
                return
            self._latest = frame
            self._changed.set()
            return
        identifier = message.get("id")
        if type(identifier) is not int:
            return
        reply = self._pending.get(identifier)
        if reply is None or reply.done():
            return
        if "error" in message:
            reply.set_exception(AgentError(str(message["error"])))
        else:
            reply.set_result(message.get("result"))

    async def _log_errors(self) -> None:
        process = self._process
        if process is None or process.stderr is None:
            return
        while True:
            try:
                line = await process.stderr.readline()
            except ValueError:
                self._logger.error("Decky My Rig agent wrote an over-long error line.")
                continue
            except ConnectionError:
                return
            if not line:
                return
            text = line.decode(errors="replace").rstrip()
            self._last_error = text
            self._logger.error("Decky My Rig agent: %s", text)

    def _fail_pending(self) -> None:
        for reply in self._pending.values():
            if not reply.done():
                reply.set_exception(AgentError(UNAVAILABLE))

    def _closed_frame(self) -> Frame:
        frame = self._latest or {
            "revision": 0,
            "pcs": [],
            "test_stops": {"count": 0, "name": None},
        }
        return {**frame, "closed": True, "refreshed": False}
