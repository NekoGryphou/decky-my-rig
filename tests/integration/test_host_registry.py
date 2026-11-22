"""Paired-device registry rules over real TCP: tokens, removal, unpairing, storage."""

import hashlib
import json
import os
import unittest
from pathlib import Path

from host_fixtures import BODY, HOST_ID, OTHER_ID, REQUEST_ID, ROOT, Harness


# noinspection PyAttributeOutsideInit,PyPep8Naming,PyMethodMayBeStatic
@unittest.skipUnless(os.name == "posix", "Harmless WSL/POSIX loopback harness")
class HostRegistryContract(unittest.TestCase):
    FIRST = "01" * 32
    SECOND = "02" * 32

    def setUp(self):
        self.host = Harness(registry_tokens=[self.FIRST, self.SECOND])
        self.expected_calls = 0

    def tearDown(self):
        if self.host is not None:
            self.assertEqual(self.host.close(), self.expected_calls)

    def headers(self, token, post=False):
        result = [
            ("Authorization", "Bearer " + token),
            ("X-MyRig-Protocol-Major", "1"),
            ("Accept", "application/json"),
        ]
        return result + ([("Content-Type", "application/json")] if post else [])

    def test_independent_tokens_on_status_and_shutdown(self):
        for token in (self.FIRST, self.SECOND):
            status, _, value = self.host.request(headers=self.headers(token))
            self.assertEqual(status, 200)
            self.assertEqual(value["host_id"], HOST_ID)
            self.assertNotIn("paired_devices", value)
            self.assertNotIn(token, json.dumps(value))
        status, _, _ = self.host.request(
            "POST", "/v1/shutdown", BODY, self.headers(self.SECOND, post=True)
        )
        self.assertEqual(status, 202)
        self.expected_calls = 1
        status, _, _ = self.host.request(
            "POST", "/v1/shutdown", BODY, self.headers("03" * 32, post=True)
        )
        self.assertEqual(status, 401)

    def test_pairing_code_request_records_one_event_over_tls(self):
        headers = [
            ("Accept", "application/json"),
            ("Content-Type", "application/json"),
            ("X-MyRig-Protocol-Major", "1"),
        ]
        body = b'{"device_name":"steamdeck"}'
        shown = self.host.request("POST", "/v1/pair/request", body, headers)
        again = self.host.request("POST", "/v1/pair/request", body, headers)
        self.assertEqual((shown[0], again[0]), (202, 409))
        self.assertEqual(
            [
                (event["kind"], event["device_name"])
                for event in self.host.test_events()
            ],
            [("pairing_request", "steamdeck")],
        )

    def test_test_mode_stop_records_no_power_call_and_stays_online(self):
        self.assertEqual(self.host.close(), 0)
        self.host = Harness(registry_tokens=[self.FIRST], test_mode=True)
        status, _, value = self.host.request(
            "POST", "/v1/shutdown", BODY, self.headers(self.FIRST, post=True)
        )
        self.assertEqual(status, 202)
        expected = json.loads(
            (ROOT / "tests/fixtures/lan-api/shutdown-test-accepted.json").read_text()
        )
        expected["request_id"] = REQUEST_ID
        self.assertEqual(value, expected)
        self.assertEqual(
            [event["device_name"] for event in self.host.test_events()], ["Steam Deck"]
        )
        status, _, value = self.host.request(headers=self.headers(self.FIRST))
        self.assertEqual(status, 200)
        self.assertEqual(value["state"], "online")

    def test_test_mode_is_read_for_each_stop_request(self):
        self.assertEqual(self.host.close(), 0)
        self.host = Harness(registry_tokens=[self.FIRST], test_mode=True)
        status, _, value = self.host.request(
            "POST", "/v1/shutdown", BODY, self.headers(self.FIRST, post=True)
        )
        self.assertEqual(status, 202)
        self.assertTrue(value["test"])
        self.host.set_test_mode(False)
        other = json.dumps({"request_id": OTHER_ID}).encode()
        status, _, value = self.host.request(
            "POST", "/v1/shutdown", other, self.headers(self.FIRST, post=True)
        )
        self.assertEqual(status, 202)
        self.assertNotIn("test", value)
        self.expected_calls = 1

    def test_removed_token_remains_rejected_after_config_reload(self):
        self.assertEqual(self.host.close(), 0)
        self.host = None
        self.host = Harness(registry_tokens=[self.SECOND])
        for method, path, body in (
            ("GET", "/v1/status", b""),
            ("POST", "/v1/shutdown", BODY),
        ):
            status, _, value = self.host.request(
                method, path, body, self.headers(self.FIRST, post=method == "POST")
            )
            self.assertEqual(status, 401)
            self.assertNotIn("host_id", value)
        self.assertEqual(self.host.request(headers=self.headers(self.SECOND))[0], 200)

    def test_local_removal_revokes_both_routes_without_stopping_other_device(self):
        self.assertTrue(
            self.host.remove_test_device("11111111-1111-4111-8111-000000000001")
        )
        for method, path, body in (
            ("GET", "/v1/status", b""),
            ("POST", "/v1/shutdown", BODY),
        ):
            self.assertEqual(
                self.host.request(
                    method, path, body, self.headers(self.FIRST, post=method == "POST")
                )[0],
                401,
            )
        self.assertEqual(self.host.request(headers=self.headers(self.SECOND))[0], 200)
        status, _, _ = self.host.request(
            "POST", "/v1/shutdown", BODY, self.headers(self.SECOND, post=True)
        )
        self.assertEqual(status, 202)
        self.expected_calls = 1
        self.assertFalse(
            self.host.remove_test_device("11111111-1111-4111-8111-000000000001")
        )

    def test_a_device_removes_only_its_own_pairing_durably(self):
        status, _, value = self.host.request(
            "DELETE", "/v1/pairing", b"", self.headers(self.FIRST)
        )
        self.assertEqual((status, value["revoked"]), (200, True))
        self.assertEqual(self.host.request(headers=self.headers(self.FIRST))[0], 401)
        self.assertEqual(self.host.request(headers=self.headers(self.SECOND))[0], 200)
        saved = json.loads((Path(self.host.directory.name) / "host.json").read_text())
        digests = [device["token_sha256"] for device in saved["paired_devices"]]
        self.assertEqual(
            digests, [hashlib.sha256(bytes.fromhex(self.SECOND)).hexdigest()]
        )
        # ! A revoked token cannot remove anything else.
        status, _, _ = self.host.request(
            "DELETE", "/v1/pairing", b"", self.headers(self.FIRST)
        )
        self.assertEqual(status, 401)

    def test_failed_unpair_save_keeps_the_pairing(self):
        self.assertEqual(self.host.close(), 0)
        self.host = Harness(registry_tokens=[self.FIRST], pair_save_fail=True)
        status, _, _ = self.host.request(
            "DELETE", "/v1/pairing", b"", self.headers(self.FIRST)
        )
        self.assertEqual(status, 500)
        self.assertEqual(self.host.request(headers=self.headers(self.FIRST))[0], 200)

    def test_unpair_route_method_and_body_rules(self):
        status, headers, _ = self.host.request(
            "GET", "/v1/pairing", b"", self.headers(self.FIRST)
        )
        self.assertEqual((status, headers.get("allow")), (405, "DELETE"))
        status, _, _ = self.host.request(
            "DELETE", "/v1/pairing", b"{}", self.headers(self.FIRST, post=True)
        )
        self.assertEqual(status, 400)
        self.assertEqual(self.host.request(headers=self.headers(self.FIRST))[0], 200)

    def test_empty_registry_authenticates_nothing_and_pairing_stays_closed(self):
        self.assertEqual(self.host.close(), 0)
        self.host = None
        self.host = Harness(registry_tokens=[])
        self.assertEqual(self.host.request(headers=self.headers(self.FIRST))[0], 401)
        self.assertEqual(
            self.host.request(
                "POST", "/v1/shutdown", BODY, self.headers(self.FIRST, post=True)
            )[0],
            401,
        )
        status, _, value = self.host.request(
            "POST",
            "/v1/pair",
            b'{"code":"12345678"}',
            [
                ("X-MyRig-Protocol-Major", "1"),
                ("Accept", "application/json"),
                ("Content-Type", "application/json"),
            ],
        )
        self.assertEqual(status, 401)
        self.assertEqual(value["error"]["code"], "unauthorized")

    def test_uncertain_storage_fails_closed_on_both_routes_for_all_devices(self):
        self.host.inject_storage_uncertainty("11111111-1111-4111-8111-000000000001")
        for token in (self.FIRST, self.SECOND):
            for method, path, body in (
                ("GET", "/v1/status", b""),
                ("POST", "/v1/shutdown", BODY),
            ):
                status, _, value = self.host.request(
                    method, path, body, self.headers(token, post=method == "POST")
                )
                self.assertEqual(status, 401)
                self.assertNotIn("host_id", value)
