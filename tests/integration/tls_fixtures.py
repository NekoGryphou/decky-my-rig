"""Generated isolated test identity, never a production certificate or key."""

import hashlib
import ssl
import subprocess
from pathlib import Path


def create_identity(directory: Path, name: str = "host") -> tuple[Path, Path, str]:
    certificate = directory / f"{name}.pem"
    key = directory / f"{name}.key"
    subprocess.run(
        [
            "openssl",
            "req",
            "-x509",
            "-newkey",
            "rsa:2048",
            "-sha256",
            "-nodes",
            "-keyout",
            str(key),
            "-out",
            str(certificate),
            "-days",
            "1",
            "-subj",
            "/CN=MyRig isolated fixture",
        ],
        check=True,
        capture_output=True,
        timeout=15,
    )
    key.chmod(0o600)
    der = ssl.PEM_cert_to_DER_cert(certificate.read_text())
    subprocess.run(
        [
            "openssl",
            "x509",
            "-in",
            str(certificate),
            "-outform",
            "DER",
            "-out",
            str(certificate.with_suffix(".der")),
        ],
        check=True,
        capture_output=True,
        timeout=5,
    )
    key_der = key.with_suffix(".key.der")
    subprocess.run(
        [
            "openssl",
            "pkcs8",
            "-topk8",
            "-nocrypt",
            "-in",
            str(key),
            "-outform",
            "DER",
            "-out",
            str(key_der),
        ],
        check=True,
        capture_output=True,
        timeout=5,
    )
    key_der.chmod(0o600)
    return certificate, key, hashlib.sha256(der).hexdigest()
