"""Actual generated identities through the strict Rust startup bundle parser."""

import os
import subprocess
import tempfile
import unittest
from pathlib import Path

from host_fixtures import ROOT
from tls_fixtures import create_identity


class SecurityBundleTests(unittest.TestCase):
    def test_generated_identity_and_real_key_mismatch(self):
        with tempfile.TemporaryDirectory(prefix="myrig-security-input-") as temporary:
            directory = Path(temporary)
            certificate, key, _ = create_identity(directory)
            other, _, _ = create_identity(directory, "other")
            environment = os.environ.copy()
            environment["CARGO_TARGET_DIR"] = str(ROOT / "out/cargo")
            environment["MY_RIG_TEST_SECURITY_CERT"] = str(
                certificate.with_suffix(".der")
            )
            environment["MY_RIG_TEST_SECURITY_KEY"] = str(key.with_suffix(".key.der"))
            environment["MY_RIG_TEST_SECURITY_OTHER_CERT"] = str(
                other.with_suffix(".der")
            )
            result = subprocess.run(
                [
                    "cargo",
                    "test",
                    "--manifest-path",
                    str(ROOT / "Cargo.toml"),
                    "--locked",
                    "-p",
                    "myrig-host-core",
                    "--lib",
                    "security_bundle::tests::generated_identity_roundtrip_and_mismatch",
                    "--",
                    "--ignored",
                    "--exact",
                ],
                env=environment,
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
                text=True,
                timeout=60,
                check=False,
            )
            self.assertEqual(result.returncode, 0, result.stdout)
            self.assertIn("1 passed", result.stdout)
