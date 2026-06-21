"""Manifest helpers for verified offload and restore-proof workflows."""

from __future__ import annotations

import datetime as dt
import hashlib
import json
import os
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Iterable

SCHEMA_VERSION = "northroot.durability.manifest.v0"


@dataclass(frozen=True)
class FileRecord:
    relative_path: str
    size_bytes: int
    sha256: str


@dataclass(frozen=True)
class TreeManifest:
    schema_version: str
    generated_at: str
    root_path: str
    file_count: int
    total_bytes: int
    files: tuple[FileRecord, ...]

    def to_json(self) -> str:
        return json.dumps(asdict(self), indent=2, sort_keys=True) + "\n"


def utc_now() -> str:
    return dt.datetime.now(tz=dt.timezone.utc).isoformat()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def iter_files(root: Path) -> Iterable[Path]:
    if root.is_file():
        yield root
        return
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames[:] = sorted(name for name in dirnames if not name.startswith("._"))
        for name in sorted(filenames):
            if name.startswith("._"):
                continue
            yield Path(dirpath) / name


def build_tree_manifest(root: Path) -> TreeManifest:
    if not root.exists():
        raise FileNotFoundError(root)
    resolved = root.resolve()
    records: list[FileRecord] = []
    total = 0
    for file_path in iter_files(resolved):
        stat = file_path.stat()
        total += stat.st_size
        relative = file_path.name if resolved.is_file() else str(file_path.relative_to(resolved))
        records.append(FileRecord(relative, stat.st_size, sha256_file(file_path)))
    return TreeManifest(
        schema_version=SCHEMA_VERSION,
        generated_at=utc_now(),
        root_path=str(resolved),
        file_count=len(records),
        total_bytes=total,
        files=tuple(records),
    )


def verify_tree_manifest(manifest: TreeManifest, copy_root: Path) -> list[str]:
    errors: list[str] = []
    base = copy_root.resolve()
    seen = 0
    total = 0
    for record in manifest.files:
        path = base if base.is_file() else base / record.relative_path
        if not path.exists():
            errors.append(f"missing:{record.relative_path}")
            continue
        stat = path.stat()
        seen += 1
        total += stat.st_size
        if stat.st_size != record.size_bytes:
            errors.append(f"size-mismatch:{record.relative_path}")
            continue
        digest = sha256_file(path)
        if digest != record.sha256:
            errors.append(f"sha256-mismatch:{record.relative_path}")
    if seen != manifest.file_count:
        errors.append(f"file-count-mismatch:expected={manifest.file_count}:actual={seen}")
    if total != manifest.total_bytes:
        errors.append(f"total-bytes-mismatch:expected={manifest.total_bytes}:actual={total}")
    return errors


def load_tree_manifest(path: Path) -> TreeManifest:
    payload = json.loads(path.read_text(encoding="utf-8"))
    if payload.get("schema_version") != SCHEMA_VERSION:
        raise ValueError(f"unsupported manifest schema: {payload.get('schema_version')}")
    return TreeManifest(
        schema_version=str(payload["schema_version"]),
        generated_at=str(payload["generated_at"]),
        root_path=str(payload["root_path"]),
        file_count=int(payload["file_count"]),
        total_bytes=int(payload["total_bytes"]),
        files=tuple(FileRecord(**item) for item in payload.get("files", [])),
    )
