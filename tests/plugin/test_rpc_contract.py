import unittest
from pathlib import Path

from rpc_contract_utils import drift, python_contract, typescript_contract

ROOT = Path(__file__).parents[2]
CONSTS = (ROOT / "src/shared/api.consts.ts").read_text()
TYPES = (ROOT / "src/shared/api.types.ts").read_text()
MAIN = (ROOT / "main.py").read_text()


class RpcContractTests(unittest.TestCase):
    def test_the_frontend_and_the_plugin_agree_on_every_method(self) -> None:
        typescript = typescript_contract(CONSTS, TYPES)
        self.assertEqual(len(typescript), 12)
        self.assertEqual(drift(typescript, python_contract(MAIN)), [])

    def test_a_method_renamed_in_typescript_is_drift(self) -> None:
        renamed = CONSTS.replace('"detect_mac"', '"find_mac"')
        self.assertNotEqual(renamed, CONSTS)
        problems = drift(typescript_contract(renamed, TYPES), python_contract(MAIN))
        self.assertEqual(len(problems), 2)

    def test_a_method_renamed_in_python_is_drift(self) -> None:
        renamed = MAIN.replace("async def detect_mac(", "async def find_mac(")
        self.assertNotEqual(renamed, MAIN)
        problems = drift(typescript_contract(CONSTS, TYPES), python_contract(renamed))
        self.assertEqual(len(problems), 2)

    def test_an_argument_renamed_on_one_side_is_drift(self) -> None:
        renamed = TYPES.replace("draftId: string, name", "draft: string, name")
        self.assertNotEqual(renamed, TYPES)
        problems = drift(typescript_contract(CONSTS, renamed), python_contract(MAIN))
        self.assertEqual(len(problems), 1)
        self.assertIn("save_pairing", problems[0])


if __name__ == "__main__":
    unittest.main()
