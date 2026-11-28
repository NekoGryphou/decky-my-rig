"""Harmless Rust host harness launched by the real-TCP tests.

Run: python3 -m unittest discover -s tests/integration -v
No production Python/Rust serializer, validation or authentication helper is imported.
"""

import hashlib
import http.client
import json
import os
import queue
import secrets
import signal
import socket
import ssl
import subprocess
import tempfile
import threading
import time
from pathlib import Path

from tls_fixtures import create_identity

ROOT = Path(__file__).resolve().parents[2]
HOST_ID = "03c85317-9d93-4fb3-a04b-91a0e16668c6"
REQUEST_ID = "d1d5303f-bfd4-4861-aaaf-e992ec82ee1b"
OTHER_ID = "a1d5303f-bfd4-4861-aaaf-e992ec82ee1b"
BODY = ('{"request_id":"' + REQUEST_ID + '"}').encode()


class Harness:
    def __init__(
        self,
        refuse=False,
        registry_tokens=None,
        pair_save_fail=False,
        tls_identity=None,
        secure_pair=False,
        approved_tls_certificate=None,
        test_mode=False,
    ):
        self.directory = tempfile.TemporaryDirectory(prefix="myrig-http-contract-")
        self.token = secrets.token_bytes(32).hex()
        tokens = [self.token] if registry_tokens is None else registry_tokens
        self.token = tokens[0] if tokens else self.token
        config = {
            "schema_version": 3,
            "host_id": HOST_ID,
            "port": 47991,
            "test_mode": test_mode,
            "paired_devices": [
                {
                    "device_id": "11111111-1111-4111-8111-" + f"{index + 1:012d}",
                    "name": "Steam Deck",
                    "token_sha256": hashlib.sha256(bytes.fromhex(token)).hexdigest(),
                }
                for index, token in enumerate(tokens)
            ],
        }
        path = Path(self.directory.name) / "host.json"
        path.write_text(json.dumps(config), encoding="utf-8")
        if tls_identity is None:
            certificate, key, _ = create_identity(Path(self.directory.name))
            tls_identity = (
                certificate.with_suffix(".der"),
                key.with_suffix(".key.der"),
            )
        self.pin = hashlib.sha256(tls_identity[0].read_bytes()).hexdigest()
        self.context = ssl.SSLContext(ssl.PROTOCOL_TLS_CLIENT)
        self.context.check_hostname = False
        self.context.verify_mode = ssl.CERT_NONE
        self.context.set_alpn_protocols(["http/1.1"])
        env = os.environ.copy()
        env.setdefault("CARGO_TARGET_DIR", str(ROOT / "out/cargo"))
        env["MY_RIG_TEST_CONFIG"] = str(path)
        env.pop("MY_RIG_TEST_REFUSE", None)
        env.pop("MY_RIG_TEST_PAIR_SAVE_FAIL", None)
        env.pop("MY_RIG_TEST_TLS_CERT", None)
        env.pop("MY_RIG_TEST_TLS_KEY", None)
        env.pop("MY_RIG_TEST_SECURE_PAIR", None)
        env.pop("MY_RIG_TEST_APPROVED_TLS_CERT", None)
        if secure_pair:
            env["MY_RIG_TEST_SECURE_PAIR"] = "1"
        if approved_tls_certificate is not None:
            env["MY_RIG_TEST_APPROVED_TLS_CERT"] = str(approved_tls_certificate)
        if tls_identity is not None:
            certificate, key = tls_identity
            env["MY_RIG_TEST_TLS_CERT"] = str(certificate)
            env["MY_RIG_TEST_TLS_KEY"] = str(key)
        if pair_save_fail:
            env["MY_RIG_TEST_PAIR_SAVE_FAIL"] = "1"
        if refuse:
            env["MY_RIG_TEST_REFUSE"] = "1"
        self.process = subprocess.Popen(
            [
                "cargo",
                "test",
                "--locked",
                "--quiet",
                "-p",
                "myrig-host-core",
                "--test",
                "http_harness",
                "serve",
                "--",
                "--ignored",
                "--nocapture",
            ],
            cwd=ROOT,
            env=env,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            start_new_session=True,
        )
        self.lines = []
        self.events = queue.Queue()
        self.reader = threading.Thread(target=self._read, daemon=True)
        self.reader.start()
        deadline = time.monotonic() + 60
        try:
            while True:
                line = self.events.get(timeout=max(0.01, deadline - time.monotonic()))
                if line is None:
                    detail = " | ".join(self.lines[-50:])
                    raise RuntimeError(
                        "Harmless host harness exited before readiness: " + detail
                    )
                if line.startswith("READY "):
                    self.port = int(line.split()[1])
                    break
        except BaseException:
            os.killpg(self.process.pid, signal.SIGKILL)
            self.process.wait(timeout=5)
            self.reader.join(timeout=2)
            self.process.stdin.close()
            self.process.stdout.close()
            self.directory.cleanup()
            raise

    def _read(self):
        for line in self.process.stdout:
            self.lines.append(line.strip())
            self.events.put(line.strip())
        self.events.put(None)

    def close(self):
        try:
            self.process.stdin.write("\n")
            self.process.stdin.flush()
            self.process.wait(timeout=8)
            self.reader.join(timeout=2)
            if self.process.returncode != 0:
                raise AssertionError("Harmless harness failed during shutdown")
            counts = [
                int(line.split()[1]) for line in self.lines if line.startswith("CALLS ")
            ]
            if len(counts) != 1:
                raise AssertionError(
                    "Harness did not report exactly one final power-call count"
                )
            return counts[0]
        finally:
            if self.process.poll() is None:
                os.killpg(self.process.pid, signal.SIGKILL)
                self.process.wait(timeout=5)
            self.process.stdin.close()
            self.process.stdout.close()
            self.directory.cleanup()

    def headers(self):
        return [
            ("Authorization", "Bearer " + self.token),
            ("X-MyRig-Protocol-Major", "1"),
            ("Accept", "application/json"),
        ]

    def remove_test_device(self, device_id):
        self.process.stdin.write("REVOKE " + device_id + "\n")
        self.process.stdin.flush()
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            line = self.events.get(timeout=max(0.01, deadline - time.monotonic()))
            if line is None:
                raise AssertionError("Harness exited during local removal")
            if line.startswith("REVOKED "):
                return line == "REVOKED true"
        raise AssertionError("Local removal did not finish within the test bound")

    def set_test_mode(self, enabled):
        self.process.stdin.write("TEST_MODE " + ("on" if enabled else "off") + "\n")
        self.process.stdin.flush()
        line = self.events.get(timeout=5)
        if line != "TEST_MODE_SET":
            raise AssertionError(
                "Harness did not finish the test mode update: " + str(line)
            )

    def control(self, request):
        self.process.stdin.write("CONTROL " + json.dumps(request) + "\n")
        self.process.stdin.flush()
        line = self.events.get(timeout=5)
        if line is None or not line.startswith("CONTROL "):
            raise AssertionError("Harness did not return local control: " + str(line))
        return json.loads(line.removeprefix("CONTROL "))

    def test_events(self):
        self.process.stdin.write("EVENTS\n")
        self.process.stdin.flush()
        line = self.events.get(timeout=5)
        if line is None or not line.startswith("EVENTS "):
            raise AssertionError("Harness did not return test events: " + str(line))
        return json.loads(line.removeprefix("EVENTS "))

    def inject_storage_uncertainty(self, device_id):
        self.process.stdin.write("UNCERTAIN_REVOKE " + device_id + "\n")
        self.process.stdin.flush()
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            line = self.events.get(timeout=max(0.01, deadline - time.monotonic()))
            if line == "RECOVERY_REQUIRED":
                return
            if line is None:
                raise AssertionError("Harness exited during storage uncertainty")
        raise AssertionError("Storage uncertainty did not finish within test bound")

    def connect(self):
        raw = socket.create_connection(("127.0.0.1", self.port), timeout=4)
        try:
            stream = self.context.wrap_socket(raw, server_hostname="127.0.0.1")
        except BaseException:
            raw.close()
            raise
        if hashlib.sha256(stream.getpeercert(binary_form=True)).hexdigest() != self.pin:
            stream.close()
            raise AssertionError("Harness TLS identity changed")
        return stream

    def connection(self):
        connection = http.client.HTTPSConnection(
            "127.0.0.1", self.port, timeout=4, context=self.context
        )
        connection.sock = self.connect()
        return connection

    def request(self, method="GET", path="/v1/status", body=b"", headers=None):
        headers = self.headers() if headers is None else headers
        connection = self.connection()
        try:
            connection.putrequest(method, path, skip_accept_encoding=True)
            for key, value in headers:
                connection.putheader(key, value)
            connection.putheader("Content-Length", str(len(body)))
            connection.putheader("Connection", "close")
            connection.endheaders(body)
            response = connection.getresponse()
            payload = response.read(16385)
            if len(payload) > 16384:
                raise AssertionError("Response exceeded contract limit")
            if self.token.encode() in payload:
                raise AssertionError("Response leaked a bearer token")
            return response.status, dict(response.getheaders()), json.loads(payload)
        finally:
            connection.close()
