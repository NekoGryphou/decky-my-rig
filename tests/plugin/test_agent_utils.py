import os
import tempfile
import unittest
from pathlib import Path

from my_rig.agent.agent_utils import agent_name, remove_saved_state


class AgentNameTests(unittest.TestCase):
    def test_each_platform_names_its_agent_like_the_release_files(self) -> None:
        for machine, system, name in [
            ("x86_64", "Linux", "myrig-x86_64-linux"),
            ("AMD64", "Windows", "myrig-x86_64-windows.exe"),
            ("aarch64", "Linux", "myrig-aarch64-linux"),
            ("riscv64", "Linux", "myrig-riscv64-linux"),
        ]:
            with self.subTest(machine=machine, system=system):
                self.assertEqual(agent_name(machine, system), name)


# noinspection PyAttributeOutsideInit,PyPep8Naming
class RemoveSavedStateTests(unittest.TestCase):
    def setUp(self) -> None:
        self.root = tempfile.TemporaryDirectory()
        self.addCleanup(self.root.cleanup)
        self.settings = Path(self.root.name) / "DeckyMyRig"

    def test_the_saved_pcs_and_their_lock_are_removed(self) -> None:
        self.settings.mkdir()
        (self.settings / "pcs.json").write_text("{}")
        (self.settings / ".pcs.lock").write_text("")
        remove_saved_state(self.settings)
        self.assertFalse(self.settings.exists())

    def test_a_missing_directory_is_already_clean(self) -> None:
        remove_saved_state(self.settings)
        self.assertFalse(self.settings.exists())

    @unittest.skipUnless(os.name == "posix", "symbolic links need no privilege")
    def test_a_linked_directory_is_refused_and_its_target_kept(self) -> None:
        target = Path(self.root.name) / "elsewhere"
        target.mkdir()
        (target / "pcs.json").write_text("{}")
        self.settings.symlink_to(target)
        with self.assertRaises(OSError):
            remove_saved_state(self.settings)
        self.assertTrue((target / "pcs.json").exists())
