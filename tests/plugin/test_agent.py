import asyncio
import importlib.util
import os
import stat
import sys
import tempfile
import types
import unittest
from pathlib import Path
from unittest.mock import patch

from my_rig.agent import AgentClient, AgentError

FIXTURE = Path(__file__).with_name("agent_fixtures.py")
UNAVAILABLE = "Decky My Rig is unavailable. Reload the plugin."


# noinspection PyTypeChecker
class Logger:
    def __init__(self) -> None:
        self.errors: list[str] = []
        self._logged = asyncio.Event()

    def error(self, message: str, *arguments: object) -> None:
        self.errors.append(message % arguments)
        self._logged.set()

    async def seen(self, text: str) -> None:
        """Returns once an error line containing `text` was logged."""
        while not any(text in line for line in self.errors):
            self._logged.clear()
            await self._logged.wait()


# noinspection PyTypeHints
def exit_code(process: asyncio.subprocess.Process) -> int | None:
    """The agent's exit code as close() leaves it, before asyncio notices."""
    transport = getattr(process, "_transport")  # noqa: B009 - asyncio internal
    code: int | None = transport.get_extra_info("subprocess").returncode
    return code


# noinspection PyAttributeOutsideInit,PyPep8Naming
class AgentTests(unittest.IsolatedAsyncioTestCase):
    async def asyncSetUp(self) -> None:
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.logger = Logger()

    def fake(self, mode: str) -> Path:
        script = Path(self.directory.name) / f"agent-{mode}"
        marker = Path(self.directory.name) / f"ran-{mode}"
        script.write_text(
            f"#!/bin/sh\nexec {sys.executable} -I {FIXTURE} {mode} {marker}\n"
        )
        script.chmod(script.stat().st_mode | stat.S_IXUSR)
        return script

    async def started(self, mode: str) -> AgentClient:
        agent = AgentClient(self.fake(mode), Path(self.directory.name), self.logger)
        await agent.start()
        self.addAsyncCleanup(agent.close)
        return agent

    async def test_calls_are_relayed_with_their_parameters(self) -> None:
        agent = await self.started("echo")
        result = await agent.call("stop", {"identifier": "x"})
        self.assertEqual(result, {"method": "stop", "params": {"identifier": "x"}})
        calls = await asyncio.gather(
            *(agent.call("settings", {"n": n}) for n in range(5))
        )
        self.assertEqual([call["params"]["n"] for call in calls], list(range(5)))

    async def test_agent_errors_reach_the_caller_as_their_message(self) -> None:
        agent = await self.started("error")
        with self.assertRaises(AgentError) as raised:
            await agent.call("start", {"identifier": "x"})
        self.assertEqual(str(raised.exception), "PC was not found.")

    async def test_lines_that_are_not_replies_are_ignored(self) -> None:
        agent = await self.started("junk")
        result = await asyncio.wait_for(agent.call("settings", {}), 5)
        self.assertEqual(result["method"], "settings")
        frame = await asyncio.wait_for(agent.watch(-1), 5)
        self.assertEqual(frame["revision"], 1)

    async def test_a_missing_agent_reports_unavailable_at_once(self) -> None:
        missing = Path(self.directory.name) / "myrig-riscv64-linux"
        agent = AgentClient(missing, Path(self.directory.name), self.logger)
        await agent.start()
        self.addAsyncCleanup(agent.close)
        frame = await asyncio.wait_for(agent.watch(-1), 5)
        self.assertTrue(frame["closed"])
        with self.assertRaises(AgentError) as raised:
            await agent.call("settings", {})
        self.assertEqual(str(raised.exception), UNAVAILABLE)
        self.assertIn("myrig-riscv64-linux", "\n".join(self.logger.errors))

    async def test_agent_exit_fails_pending_calls(self) -> None:
        agent = await self.started("exit")
        for _ in range(2):
            with self.assertRaises(AgentError) as raised:
                await asyncio.wait_for(agent.call("settings", {}), 5)
            self.assertEqual(str(raised.exception), UNAVAILABLE)

    async def test_unload_while_pairing_leaves_no_process(self) -> None:
        agent = await self.started("hang")
        pairing = asyncio.create_task(agent.call("begin_pairing", {}))
        await asyncio.wait_for(self.logger.seen("pairing received"), 5)
        await asyncio.wait_for(agent.close(), 6)
        with self.assertRaises(AgentError):
            await pairing
        process = agent._process
        assert process is not None
        self.assertIsNotNone(exit_code(process))

    async def test_close_finishes_without_waiting_on_the_event_loop(self) -> None:
        # ! When Decky stops a plugin, its socket listener spins on EOF without
        #   yielding, so a close that awaits anything never gets to continue.
        agent = await self.started("echo")
        process = agent._process
        assert process is not None
        closing = agent.close()
        with self.assertRaises(StopIteration):
            closing.send(None)
        self.assertIsNotNone(exit_code(process))

    async def test_an_agent_ignoring_eof_is_stopped(self) -> None:
        agent = await self.started("linger")
        with patch("my_rig.agent.agent.EXIT_TIMEOUT_SECONDS", 0.2):
            await asyncio.wait_for(agent.close(), 5)
        process = agent._process
        assert process is not None
        self.assertIsNotNone(exit_code(process))
        self.assertTrue(any("did not exit" in line for line in self.logger.errors))

    async def test_close_stops_an_agent_whose_replies_stopped(self) -> None:
        # - The reader gave up on an over-long line while the agent lives on.
        with patch("my_rig.agent.agent.LINE_LIMIT", 64):
            agent = await self.started("oversize")
            frame = await asyncio.wait_for(agent.watch(-1), 10)
            self.assertTrue(frame["closed"])
            process = agent._process
            assert process is not None
            await asyncio.wait_for(agent.close(), 6)
        self.assertIsNotNone(exit_code(process))

    async def test_a_crashed_agent_is_started_again(self) -> None:
        agent = await self.started("crash-once")
        frame = await asyncio.wait_for(agent.watch(-1), 5)
        self.assertFalse(frame["closed"])
        result = await asyncio.wait_for(agent.call("settings", {}), 5)
        self.assertEqual(result["method"], "settings")
        self.assertTrue(any("restart" in line for line in self.logger.errors))

    async def test_a_refusing_agent_reports_its_reason_and_is_not_restarted(
        self,
    ) -> None:
        agent = await self.started("refuse")
        frame = await asyncio.wait_for(agent.watch(-1), 5)
        self.assertTrue(frame["closed"])
        with self.assertRaises(AgentError) as raised:
            await agent.call("settings", {})
        self.assertEqual(
            str(raised.exception),
            "Cannot load PC configuration; existing data was not reset.",
        )
        self.assertFalse(any("restart" in line for line in self.logger.errors))

    async def test_an_over_long_error_line_does_not_stop_error_logging(self) -> None:
        with patch("my_rig.agent.agent.LINE_LIMIT", 64):
            await self.started("noisy")
            await asyncio.wait_for(self.logger.seen("after the long line"), 5)

    async def test_watch_long_polls_and_close_releases_watchers(self) -> None:
        agent = await self.started("echo")
        first = await asyncio.wait_for(agent.watch(-1), 5)
        self.assertEqual(first["revision"], 1)
        waiting = asyncio.create_task(agent.watch(1))
        # - Let every other task run; a watch that doesn't wait would be done by
        #   now.
        for _ in range(50):
            await asyncio.sleep(0)
        self.assertFalse(waiting.done())
        for invalid in (-2, "1", True, None):
            with self.assertRaises(AgentError):
                await agent.watch(invalid)
        await agent.close()
        closed = await asyncio.wait_for(waiting, 5)
        self.assertTrue(closed["closed"])
        self.assertEqual(closed["revision"], 1)
        with self.assertRaises(AgentError):
            await agent.call("settings", {})

    async def test_plugin_entry_point_forwards_each_rpc(self) -> None:
        decky = types.ModuleType("decky")
        setattr(decky, "DECKY_PLUGIN_SETTINGS_DIR", self.directory.name)
        setattr(decky, "logger", self.logger)
        with patch.dict(sys.modules, {"decky": decky}):
            spec = importlib.util.spec_from_file_location(
                "plugin_main", Path(__file__).parents[2] / "main.py"
            )
            assert spec is not None and spec.loader is not None
            main = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(main)
        with patch.object(main, "_BINARY", self.fake("echo")):
            plugin = main.Plugin()
        await plugin._main()
        self.addAsyncCleanup(plugin._unload)
        self.assertEqual(
            await plugin.begin_pairing("d", "h", 47991, "12345678", "i"),
            {
                "method": "begin_pairing",
                "params": {
                    "draft_id": "d",
                    "host": "h",
                    "port": 47991,
                    "code": "12345678",
                    "identifier": "i",
                },
            },
        )
        self.assertEqual(
            (await plugin.save_pairing("d", "n", "m"))["params"],
            {"draft_id": "d", "name": "n", "mac": "m"},
        )
        self.assertEqual(
            (await plugin.update_pc("i", {"name": "n"}))["params"],
            {"identifier": "i", "changes": {"name": "n"}},
        )
        for method in ("start", "stop", "delete_pc"):
            self.assertEqual((await getattr(plugin, method)("i"))["method"], method)
        self.assertEqual(
            (await plugin.set_attention(True))["params"], {"visible": True}
        )
        self.assertEqual((await plugin.watch_snapshot(-1))["revision"], 1)

    @unittest.skipUnless(
        os.environ.get("MYRIG_BINARY"), "set MYRIG_BINARY to a built myrig"
    )
    async def test_the_real_agent_serves_and_exits(self) -> None:
        store = Path(self.directory.name) / "settings"
        agent = AgentClient(Path(os.environ["MYRIG_BINARY"]), store, self.logger)
        await agent.start()
        self.assertEqual(await agent.call("settings", {}), [])
        with self.assertRaises(AgentError) as raised:
            await agent.call("start", {"identifier": "bad"})
        self.assertEqual(str(raised.exception), "Invalid PC identifier.")
        frame = await asyncio.wait_for(agent.watch(-1), 5)
        self.assertEqual(frame["pcs"], [])
        await agent.close()
        process = agent._process
        assert process is not None
        self.assertEqual(exit_code(process), 0)
        self.assertEqual(self.logger.errors, [])


if __name__ == "__main__":
    unittest.main()
