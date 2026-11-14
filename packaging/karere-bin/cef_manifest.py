#!/usr/bin/env python3
"""Download precisely the x86_64 CEF binary pinned by an upstream Flatpak manifest."""

from __future__ import annotations

import hashlib
import re
import sys
import urllib.parse
import urllib.request
from pathlib import Path


def main() -> None:
    if len(sys.argv) != 4:
        raise SystemExit("usage: cef_manifest.py SOURCE_DIR DESTINATION_ZIP ARCH")

    root = Path(sys.argv[1])
    destination = Path(sys.argv[2])
    arch = sys.argv[3]
    if arch != "x86_64":
        raise SystemExit(f"unsupported architecture: {arch}")

    manifest = (root / "packaging/io.github.tobagin.karere.yml").read_text()
    match = re.search(
        r"(?ms)^  - name: cef-binaries[ \t]*\n(.*?)(?=^  - name:|\Z)",
        manifest,
    )
    if match is None:
        raise SystemExit("could not locate CEF module in Flatpak manifest")

    archives = re.findall(
        r"(?ms)^      - type: archive[ \t]*\n(.*?)(?=^      - type:|\Z)",
        match.group(1),
    )
    choices: list[tuple[str, str]] = []
    for body in archives:
        only_arches = re.search(r"(?m)^\s*only-arches:\s*\[([^]]+)\]\s*$", body)
        if not only_arches:
            raise SystemExit("CEF archive is missing only-arches")
        if [a.strip() for a in only_arches.group(1).split(",")] != [arch]:
            continue
        url = re.search(r"(?m)^\s*url:\s*(\S+)\s*$", body)
        sha = re.search(r"(?m)^\s*sha256:\s*([a-fA-F0-9]{64})\s*$", body)
        if not url or not sha:
            raise SystemExit("CEF x86_64 archive needs a URL and SHA256")
        choices.append((url.group(1), sha.group(1).lower()))

    if len(choices) != 1:
        raise SystemExit(f"expected one x86_64 CEF archive, found {len(choices)}")

    url, expected_sha = choices[0]
    version_match = re.search(
        r'^CEF_VERSION="\$\{CEF_VERSION:-([^}]+)\}"$',
        (root / "download-cef.sh").read_text(),
        re.MULTILINE,
    )
    if not version_match:
        raise SystemExit("could not read upstream CEF_VERSION")

    expected_name = f"cef_binary_{version_match.group(1)}_linux64_minimal.zip"
    decoded_url = urllib.parse.unquote(url)
    if not re.fullmatch(
        r"https://github\.com/tobagin/karere/releases/download/"
        r"cef-[0-9]+\.[0-9]+\.[0-9]+-proprietary-codecs(?:-[0-9]+)?/"
        r"cef_binary_[^/]+_linux64_minimal\.zip",
        decoded_url,
    ):
        raise SystemExit(f"unexpected CEF download URL: {decoded_url}")
    if decoded_url.rsplit("/", 1)[-1] != expected_name:
        raise SystemExit(
            f"CEF version mismatch: manifest={decoded_url.rsplit('/', 1)[-1]}, "
            f"download-cef.sh={expected_name}"
        )

    destination.parent.mkdir(parents=True, exist_ok=True)
    request = urllib.request.Request(
        url,
        headers={"User-Agent": "karere-bin-github-actions"},
    )
    sha256 = hashlib.sha256()
    try:
        with urllib.request.urlopen(request, timeout=120) as response:
            with destination.open("wb") as target:
                while block := response.read(1024 * 1024):
                    target.write(block)
                    sha256.update(block)
    except BaseException:
        destination.unlink(missing_ok=True)
        raise
    actual_sha = sha256.hexdigest()
    if actual_sha != expected_sha:
        destination.unlink(missing_ok=True)
        raise SystemExit(
            f"CEF SHA256 mismatch: expected {expected_sha}, got {actual_sha}"
        )
    print(f"Verified upstream CEF: {expected_name} ({actual_sha})")


if __name__ == "__main__":
    main()
