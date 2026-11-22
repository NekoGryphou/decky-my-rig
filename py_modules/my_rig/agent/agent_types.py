from typing import Any, Protocol

# - A snapshot frame, relayed to the frontend as the agent sent it.
Frame = dict[str, Any]


class Logger(Protocol):
    """Decky's plugin logger, as far as this client uses it."""

    def error(self, message: str, *arguments: object) -> None: ...
