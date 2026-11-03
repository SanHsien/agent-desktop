#!/usr/bin/env python3
"""Check the product-file ledger against Git without changing repository state."""

import argparse
import re
import subprocess
import sys
from pathlib import Path, PurePosixPath

REPO_ROOT = Path(__file__).resolve().parents[1]
PRODUCT_PATHS = ("crates", "src", "apps", "npm")


def read_ledger(text: str) -> set[str]:
    paths = set()
    table_found = False
    in_table = False
    for line in text.splitlines():
        if line == "| Path | Reason | Removal condition |":
            table_found = in_table = True
            continue
        if not in_table:
            continue
        if not line.startswith("|"):
            in_table = False
            continue
        cells = [cell.strip() for cell in line.strip("|").split("|")]
        if all(re.fullmatch(r":?-+:?", cell) for cell in cells):
            continue
        if len(cells) != 3 or not all(cells):
            raise ValueError(
                f"each row needs a path, reason and removal condition: {line}"
            )
        match = re.fullmatch(r"`([^`]+)`", cells[0])
        if not match:
            raise ValueError(f"invalid path cell: {cells[0]}")
        name = match[1]
        path = PurePosixPath(name)
        if (
            path.parts[0] not in PRODUCT_PATHS
            or len(path.parts) < 2
            or ".." in path.parts
            or "\\" in name
            or path.as_posix() != name
        ):
            raise ValueError(f"invalid product path: {name}")
        if name in paths:
            raise ValueError(f"duplicate path: {name}")
        paths.add(name)
    if not table_found:
        raise ValueError("missing product ledger table")
    return paths


def run_git(repo: Path, *args: str) -> str:
    return subprocess.run(
        ["git", *args], cwd=repo, check=True, capture_output=True,
        text=True, encoding="utf-8", errors="strict", timeout=30,
    ).stdout


def resolve_base(repo: Path, requested: str | None, text: str) -> str:
    if requested:
        return run_git(repo, "rev-parse", "--verify", "--end-of-options",
                       f"{requested}^{{commit}}").strip()
    match = re.search(r"^Base: `([0-9a-f]{40})`$", text, re.MULTILINE)
    if not match:
        raise ValueError("the ledger has no pinned Base")
    return run_git(repo, "rev-parse", "--verify", f"{match[1]}^{{commit}}").strip()


def changed_paths(repo: Path, base: str, target: str | None) -> set[str]:
    refs = [base]
    if target:
        refs.append(run_git(repo, "rev-parse", "--verify", "--end-of-options",
                            f"{target}^{{commit}}").strip())
    output = run_git(repo, "diff", "--name-only", "--no-renames", "-z",
                     *refs, "--", *PRODUCT_PATHS)
    paths = set(filter(None, output.split("\0")))
    if target is None:
        output = run_git(repo, "ls-files", "--others", "--exclude-standard",
                         "-z", "--", *PRODUCT_PATHS)
        paths.update(filter(None, output.split("\0")))
    return paths


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, default=REPO_ROOT)
    parser.add_argument("--document", type=Path, default=Path("docs/DIVERGENCE.md"))
    parser.add_argument("--base", help="commit/ref to compare against; default the ledger's pinned Base")
    parser.add_argument("--target", help="commit/ref, e.g. HEAD; default working tree")
    args = parser.parse_args(argv)
    try:
        text = (args.repo / args.document).read_text(encoding="utf-8")
        listed = read_ledger(text)
        base = resolve_base(args.repo, args.base, text)
        actual = changed_paths(args.repo, base, args.target)
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        print(f"INCOMPLETE: {error}", file=sys.stderr)
        return 2
    print(f"base: {base}; target: {args.target or 'working tree'}")
    for name in sorted(actual - listed):
        print(f"unlisted: {name}")
    for name in sorted(listed - actual):
        print(f"obsolete: {name}")
    if actual != listed:
        return 1
    print(f"PASS: {len(actual)} product divergence paths match the ledger")
    return 0


if __name__ == "__main__":
    sys.exit(main())
