"""HTTP rules over real TCP: authentication, headers, bodies, connection limits."""

import concurrent.futures
import json
import os
import time
import unittest

from host_fixtures import BODY, HOST_ID, OTHER_ID, REQUEST_ID, Harness


# noinspection PyAttributeOutsideInit,PyPep8Naming,PyMethodMayBeStatic
@unittest.skipUnless(os.name == "posix", "Harmless WSL/POSIX loopback harness")
class HostHttpContract(unittest.TestCase):
    def setUp(self):
        self.host = Harness(
            refuse=self._testMethodName == "test_refusal_does_not_leave_pending"
        )

    def tearDown(self):
        if self.host is not None:
            self.host.close()

    def assert_error(self, response, status, code):
        actual_status, headers, data = response
        self.assertEqual(actual_status, status)
        self.assertTrue(
            headers.get("content-type", headers.get("Content-Type", "")).startswith(
                "application/json"
            )
        )
        self.assertEqual(data["protocol_version"], {"major": 1, "minor": 0})
        self.assertEqual(data["error"]["code"], code)
        self.assertIsInstance(data["error"]["message"], str)
        self.assertNotIn("host_id", data)
        self.assertNotIn("app_version", data)
        self.assertNotIn("state", data)
        if code != "unsupported_protocol":
            self.assertNotIn("supported_protocol_majors", data)
        return headers, data

    def post(self, body=BODY, extra=None):
        return self.host.request(
            "POST",
            "/v1/shutdown",
            body,
            self.host.headers()
            + [("Content-Type", "application/json")]
            + (extra or []),
        )

    def test_status_and_application_version_are_separate(self):
        status, _, data = self.host.request()
        self.assertEqual(status, 200)
        self.assertEqual(data["host_id"], HOST_ID)
        self.assertEqual(data["protocol_version"], {"major": 1, "minor": 0})
        self.assertEqual(data["state"], "online")
        self.assertIsInstance(data["app_version"], str)
        self.assertTrue(data["app_version"])

    def test_authentication_precedes_diagnostics_on_both_routes(self):
        for method, path, body in (
            ("GET", "/v1/status", b""),
            ("POST", "/v1/shutdown", b"not json"),
        ):
            for token in (None, "bad", "A" * 64, "0" * 64):
                with self.subTest(method=method, credential_case=token is None):
                    headers = (
                        [] if token is None else [("Authorization", "Bearer " + token)]
                    )
                    reply = self.host.request(method, path, body, headers)
                    hdr, _ = self.assert_error(reply, 401, "unauthorized")
                    self.assertEqual(
                        hdr.get("www-authenticate", hdr.get("WWW-Authenticate")),
                        "Bearer",
                    )
        self.assertEqual(self.host.request()[0], 200)

    def test_duplicate_authorization_is_rejected(self):
        headers = self.host.headers() + [("Authorization", "Bearer " + self.host.token)]
        self.assert_error(self.host.request(headers=headers), 401, "unauthorized")

    def test_a_bad_token_does_not_lock_out_a_valid_one(self):
        for _ in range(24):
            self.assert_error(self.host.request(headers=[]), 401, "unauthorized")
        self.assertEqual(self.host.request()[0], 200)

    def test_protocol_header_precedence_and_syntax(self):
        rest = [
            (key, value)
            for key, value in self.host.headers()
            if key != "X-MyRig-Protocol-Major"
        ]
        for value in (None, "0", "01", "+1", "1.0", "1 0", "true"):
            with self.subTest(major=value):
                headers = rest + (
                    [] if value is None else [("X-MyRig-Protocol-Major", value)]
                )
                self.assert_error(
                    self.host.request(path="/missing", headers=headers),
                    400,
                    "invalid_request",
                )
        self.assert_error(
            self.host.request(
                headers=self.host.headers() + [("X-MyRig-Protocol-Major", "1")]
            ),
            400,
            "invalid_request",
        )
        _, data = self.assert_error(
            self.host.request(headers=rest + [("X-MyRig-Protocol-Major", "2")]),
            400,
            "unsupported_protocol",
        )
        self.assertEqual(data["supported_protocol_majors"], [1])
        self.assert_error(self.host.request(path="/v2/status"), 400, "invalid_request")

    def test_routes_methods_accept_queries_and_get_body(self):
        self.assert_error(self.host.request(path="/missing"), 404, "not_found")
        headers, _ = self.assert_error(
            self.host.request("POST", "/v1/status"), 405, "method_not_allowed"
        )
        self.assertIn("GET", headers.get("allow", headers.get("Allow", "")))
        self.assert_error(
            self.host.request(path="/v1/status?debug=1"), 400, "invalid_request"
        )
        self.assert_error(self.host.request(body=b"{}"), 400, "invalid_request")
        for accept in (None, "text/plain", "application/json;q=0"):
            headers = [(k, v) for k, v in self.host.headers() if k != "Accept"]
            if accept is not None:
                headers.append(("Accept", accept))
            self.assert_error(
                self.host.request(headers=headers), 400, "invalid_request"
            )
        headers = [(k.lower(), v) for k, v in self.host.headers()]
        self.assertEqual(self.host.request(headers=headers)[0], 200)

    def test_invalid_json_and_request_ids(self):
        bodies = [
            b"{",
            b"[]",
            b"null",
            b'"text"',
            b'{"request_id":true}',
            b'{"request_id":42}',
            b"{}",
            ('{"request_id":" ' + REQUEST_ID + '"}').encode(),
            ('{"request_id":"' + REQUEST_ID.upper() + '"}').encode(),
            (
                '{"request_id":"' + REQUEST_ID + '","request_id":"' + REQUEST_ID + '"}'
            ).encode(),
            ('{"request_id":"' + REQUEST_ID + '","extra":{"a":1,"a":2}}').encode(),
            ('{"request_id":"' + REQUEST_ID + '","extra":NaN}').encode(),
            b'{"request_id":"\xff"}',
        ]
        for body in bodies:
            with self.subTest(case=bodies.index(body)):
                self.assert_error(self.post(body), 400, "invalid_request")

    def test_body_limit_and_media_type(self):
        self.assert_error(
            self.host.request("POST", "/v1/shutdown", BODY),
            415,
            "unsupported_media_type",
        )
        self.assert_error(
            self.host.request(
                "POST",
                "/v1/shutdown",
                BODY,
                self.host.headers() + [("Content-Type", "text/plain")],
            ),
            415,
            "unsupported_media_type",
        )
        self.assert_error(self.post(b" " * 4097), 413, "payload_too_large")
        padded = BODY + b" " * (4096 - len(BODY))
        self.assertEqual(self.post(padded)[0], 202)

    def test_additive_fields_and_utf8_charset(self):
        body = (
            '{"request_id":"' + REQUEST_ID + '","future":{"nested":[1,true,null]}}'
        ).encode()
        headers = self.host.headers() + [
            ("Content-Type", "application/json; charset=utf-8")
        ]
        self.assertEqual(
            self.host.request("POST", "/v1/shutdown", body, headers)[0], 202
        )

    def test_concurrent_duplicate_shutdown_calls_power_once(self):
        # ? Four, because the host serves at most four connections per address.
        with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:
            replies = list(pool.map(lambda _: self.post(), range(4)))
        expected = {
            "protocol_version": {"major": 1, "minor": 0},
            "request_id": REQUEST_ID,
            "state": "shutting_down",
        }
        for status, _, data in replies:
            self.assertEqual(status, 202)
            self.assertEqual(data, expected)
        self.assertEqual(self.host.request()[2]["state"], "shutting_down")
        self.assert_error(
            self.post(('{"request_id":"' + OTHER_ID + '"}').encode()),
            409,
            "shutdown_in_progress",
        )
        count = self.host.close()
        self.host = None
        self.assertEqual(count, 1)

    def test_refusal_does_not_leave_pending(self):
        for _ in range(2):
            self.assert_error(self.post(), 503, "shutdown_failed")
            self.assertEqual(self.host.request()[2]["state"], "online")
        count = self.host.close()
        self.host = None
        self.assertEqual(count, 2)

    def test_chunked_body_cannot_bypass_size_limit(self):
        connection = self.host.connection()
        try:
            headers = dict(self.host.headers())
            headers["Content-Type"] = "application/json"
            headers["Connection"] = "close"
            connection.request(
                "POST",
                "/v1/shutdown",
                iter([b" " * 2048, b" " * 2049]),
                headers,
                encode_chunked=True,
            )
            response = connection.getresponse()
            self.assertEqual(response.status, 413)
            self.assertEqual(
                json.loads(response.read(16384))["error"]["code"], "payload_too_large"
            )
        finally:
            connection.close()

    def test_stalled_headers_and_body_connections_close(self):
        full_headers = (
            "POST /v1/shutdown HTTP/1.1\r\nHost: localhost\r\n"
            "Authorization: Bearer " + self.host.token + "\r\n"
            "X-MyRig-Protocol-Major: 1\r\nAccept: application/json\r\n"
            "Content-Type: application/json\r\nContent-Length: 100\r\n\r\n{"
        ).encode()
        for payload in (b"GET /v1/status HTTP/1.1\r\nHost:", full_headers):
            with self.subTest(
                phase="headers" if payload.startswith(b"GET") else "body"
            ):
                started = time.monotonic()
                with self.host.connect() as stream:
                    stream.settimeout(3.5)
                    stream.sendall(payload)
                    # ? Failures at the transport level can close without a JSON
                    #   response.
                    total = 0
                    while True:
                        try:
                            part = stream.recv(4096)
                        except ConnectionResetError:
                            break
                        if not part:
                            break
                        total += len(part)
                        self.assertLessEqual(total, 16384)
                self.assertLess(time.monotonic() - started, 3.5)
        self.assertEqual(self.host.request()[0], 200)

    def test_shutdown_closes_incomplete_connections(self):
        streams = [self.host.connect() for _ in range(4)]
        try:
            for stream in streams:
                stream.sendall(b"GET /v1/status HTTP/1.1\r\n")
            started = time.monotonic()
            count = self.host.close()
            self.host = None
            self.assertEqual(count, 0)
            self.assertLess(time.monotonic() - started, 6)
        finally:
            for stream in streams:
                stream.close()


if __name__ == "__main__":
    unittest.main()


# noinspection PyAttributeOutsideInit,PyPep8Naming,PyMethodMayBeStatic
@unittest.skipUnless(os.name == "posix", "Harmless WSL/POSIX loopback harness")
class PairingHttpContract(unittest.TestCase):
    """The unauthenticated pairing routes, with a pairing session open."""

    START = "/v1/pair/opaque/start"
    FINISH = "/v1/pair/opaque/finish"
    INFO = "/v1/pair/opaque/info"

    def setUp(self):
        self.host = Harness(secure_pair=True)

    def tearDown(self):
        self.host.close()

    def headers(self, json_body=True):
        headers = [("X-MyRig-Protocol-Major", "1"), ("Accept", "application/json")]
        return headers + ([("Content-Type", "application/json")] if json_body else [])

    def assert_invalid(self, response):
        status, _, data = response
        self.assertEqual((status, data["error"]["code"]), (400, "invalid_request"))

    def test_info_requires_a_plain_get(self):
        status, _, data = self.host.request("GET", self.INFO, b"", self.headers(False))
        self.assertEqual((status, data["bootstrap_version"]), (200, 1))
        bearer = ("Authorization", "Bearer " + self.host.token)
        for method, path, body, headers in (
            ("POST", self.INFO, b"", self.headers()),
            ("GET", self.INFO + "?debug=1", b"", self.headers(False)),
            ("GET", self.INFO, b"{}", self.headers(False)),
            ("GET", self.INFO, b"", self.headers(False) + [bearer]),
            ("GET", self.INFO, b"", [("Accept", "application/json")]),
            ("GET", self.START, b"", self.headers(False)),
        ):
            with self.subTest(method=method, path=path, body=body):
                self.assert_invalid(self.host.request(method, path, body, headers))

    def test_malformed_start_and_finish_bodies_are_refused(self):
        request = "0a" * 96
        attempt, confirmation, token = "01" * 32, "04" * 64, "05" * 32
        cases = [
            (self.START, b"{"),
            (self.START, b"[]"),
            (self.START, b'{"request":3}'),
            (self.START, json.dumps({"request": request[:-2]}).encode()),
            (self.START, json.dumps({"request": request.upper()}).encode()),
            (self.START, json.dumps({"request": request, "extra": 1}).encode()),
            (
                self.START,
                f'{{"request":"{request}","request":"{request}"}}'.encode(),
            ),
            (self.FINISH, b"{}"),
            (self.FINISH, json.dumps({"attempt_id": attempt}).encode()),
            (
                self.FINISH,
                json.dumps(
                    {"attempt_id": attempt[:-2], "confirmation": confirmation}
                ).encode(),
            ),
            (
                self.FINISH,
                json.dumps(
                    {"attempt_id": attempt, "confirmation": confirmation, "extra": 1}
                ).encode(),
            ),
            (
                self.FINISH,
                json.dumps(
                    {
                        "attempt_id": attempt,
                        "confirmation": confirmation,
                        "previous_token": token[:-2],
                    }
                ).encode(),
            ),
        ]
        for path, body in cases:
            with self.subTest(path=path, body=body):
                self.assert_invalid(
                    self.host.request("POST", path, body, self.headers())
                )
        self.assert_invalid(
            self.host.request(
                "POST",
                self.START,
                json.dumps({"request": request}).encode(),
                self.headers(False),
            )
        )
