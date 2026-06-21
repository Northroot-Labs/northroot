"""Northroot durability policy CLI."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Sequence

from .manifest import build_tree_manifest, load_tree_manifest, verify_tree_manifest
from .naming import canonical_roots, normalize_root_name
from .policy import all_mode_policies, public_commit_report


def write_output(payload: object, output: str | None) -> None:
    text = json.dumps(payload, indent=2, sort_keys=True) + "\n"
    if output:
        path = Path(output)
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")
    else:
        sys.stdout.write(text)


def parse_args(argv: Sequence[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)

    sub.add_parser("roots", help="Print canonical durability root names.")
    modes = sub.add_parser("modes", help="Print tiered backup mode policy.")
    modes.add_argument("--output")

    normalize = sub.add_parser("normalize-root", help="Normalize a legacy durability folder name.")
    normalize.add_argument("name")

    public = sub.add_parser("public-check", help="Evaluate a JSON list of artifact classifications.")
    public.add_argument("manifest", help="JSON file containing a list of artifact objects.")
    public.add_argument("--output")

    manifest = sub.add_parser("manifest", help="Build a hash manifest for a source tree.")
    manifest.add_argument("path")
    manifest.add_argument("--output")

    verify = sub.add_parser("verify-manifest", help="Verify a copied tree against a manifest.")
    verify.add_argument("manifest")
    verify.add_argument("copy_root")
    verify.add_argument("--output")
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(list(argv or sys.argv[1:]))
    if args.command == "roots":
        write_output(canonical_roots(), None)
        return 0
    if args.command == "modes":
        write_output(all_mode_policies(), args.output)
        return 0
    if args.command == "normalize-root":
        print(normalize_root_name(args.name))
        return 0
    if args.command == "public-check":
        items = json.loads(Path(args.manifest).read_text(encoding="utf-8"))
        if not isinstance(items, list):
            raise ValueError("public-check manifest must be a JSON list")
        report = public_commit_report(items)
        write_output(report, args.output)
        return 1 if any(not item["allowed"] for item in report) else 0
    if args.command == "manifest":
        manifest = build_tree_manifest(Path(args.path))
        if args.output:
            Path(args.output).write_text(manifest.to_json(), encoding="utf-8")
        else:
            sys.stdout.write(manifest.to_json())
        return 0
    if args.command == "verify-manifest":
        manifest = load_tree_manifest(Path(args.manifest))
        errors = verify_tree_manifest(manifest, Path(args.copy_root))
        write_output({"ok": not errors, "errors": errors}, args.output)
        return 1 if errors else 0
    raise AssertionError(args.command)


if __name__ == "__main__":
    raise SystemExit(main())
