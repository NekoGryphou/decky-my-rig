"""Decky entry point: every call goes to the plugin's `myrig agent`."""

import os
import platform
import sys
from pathlib import Path
from typing import Any

_MODULE_DIRECTORY = os.path.join(os.path.dirname(__file__), "py_modules")
if _MODULE_DIRECTORY not in sys.path:
    sys.path.insert(0, _MODULE_DIRECTORY)

import decky
from my_rig.agent import AgentClient
from my_rig.agent.agent_utils import agent_name, remove_saved_state

# ? If the file is missing, this platform wasn't built; starting the agent reports
#   its name.
_BINARY = (
    Path(__file__).resolve().parent
    / "bin"
    / agent_name(platform.machine(), platform.system())
)


class Plugin:
    def __init__(self) -> None:
        self._settings = Path(decky.DECKY_PLUGIN_SETTINGS_DIR)
        self._agent = AgentClient(_BINARY, self._settings, decky.logger)

    async def _main(self) -> None:
        await self._agent.start()

    async def _unload(self) -> None:
        await self._agent.close()

    async def _uninstall(self) -> None:
        # ! Decky only calls this on uninstall, after _unload. Updates keep the
        #   saved PCs.
        try:
            remove_saved_state(self._settings)
        except OSError as error:
            decky.logger.error("Saved PCs were not removed: %s", error)

    async def watch_snapshot(self, after: object) -> Any:
        return await self._agent.watch(after)

    async def set_attention(self, visible: object) -> Any:
        return await self._agent.call("set_attention", {"visible": visible})

    async def settings(self) -> Any:
        return await self._agent.call("settings", {})

    async def begin_pairing(
        self,
        draft_id: object,
        host: object,
        port: object,
        code: object,
        identifier: object = None,
    ) -> Any:
        params = {"draft_id": draft_id, "host": host, "port": port, "code": code}
        if identifier is not None:
            params["identifier"] = identifier
        return await self._agent.call("begin_pairing", params)

    async def request_pairing_code(self, host: object, port: object) -> Any:
        return await self._agent.call(
            "request_pairing_code", {"host": host, "port": port}
        )

    async def cancel_pairing(self, draft_id: object) -> Any:
        return await self._agent.call("cancel_pairing", {"draft_id": draft_id})

    async def save_pairing(self, draft_id: object, name: object, mac: object) -> Any:
        return await self._agent.call(
            "save_pairing", {"draft_id": draft_id, "name": name, "mac": mac}
        )

    async def detect_mac(self, host: object) -> Any:
        return await self._agent.call("detect_mac", {"host": host})

    async def update_pc(self, identifier: object, changes: object) -> Any:
        return await self._agent.call(
            "update_pc", {"identifier": identifier, "changes": changes}
        )

    async def delete_pc(self, identifier: object) -> Any:
        return await self._agent.call("delete_pc", {"identifier": identifier})

    async def start(self, identifier: object) -> Any:
        return await self._agent.call("start", {"identifier": identifier})

    async def stop(self, identifier: object) -> Any:
        return await self._agent.call("stop", {"identifier": identifier})
