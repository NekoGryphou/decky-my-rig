import shutil
from pathlib import Path

from .agent_consts import ARCHITECTURES


def agent_name(machine: str, system: str) -> str:
    """The agent file for this platform; the zip ships one per built platform."""
    machine, system = machine.lower(), system.lower()
    name = f"myrig-{ARCHITECTURES.get(machine, machine)}-{system}"
    return name + ".exe" if system == "windows" else name


def remove_saved_state(directory: Path) -> None:
    """Deletes the saved PCs and their credentials; a link is refused, not followed."""
    try:
        shutil.rmtree(directory)
    except FileNotFoundError:
        pass
