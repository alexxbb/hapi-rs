#!/usr/bin/env python3
"""
Run `cargo xtask coverage` with LCOV export, then emit marked sources for LLM review.

Uses cargo-llvm-cov's `--lcov` export (llvm-cov export -format=lcov). Per the LCOV tracefile
format, instrumented lines appear as `DA:<line>,<execution_count>`; lines with a zero count
were never executed by tests.

Marked files are written under `target/llvm-cov/parsed/`, mirroring paths under `lib/src/`.
Uncovered lines (`DA:<line>,0`) get a trailing `// 0` comment so the output stays valid Rust.

See: https://llvm.org/docs/CommandGuide/llvm-cov.html (export -format=lcov)
     https://github.com/taiki-e/cargo-llvm-cov
"""

from __future__ import annotations

import argparse
import subprocess
import sys
from collections import defaultdict
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent
LIB_SRC = REPO_ROOT / "lib" / "src"
DEFAULT_LCOV = REPO_ROOT / "target" / "llvm-cov" / "lcov.info"
DEFAULT_PARSED = REPO_ROOT / "target" / "llvm-cov" / "parsed"
DEFAULT_IGNORE_REGEX = r"(/tests/|/build\.rs|/ffi/bindings\.rs|rustc/)"


def run_test_coverage_lcov(lcov_path: Path, ignore_regex: str) -> None:
    lcov_path.parent.mkdir(parents=True, exist_ok=True)
    cmd = [
        "cargo",
        "xtask",
        "coverage",
        "--",
        "--lcov",
        "--output-path",
        str(lcov_path),
        "--ignore-filename-regex",
        ignore_regex,
    ]
    print(f"Running: {' '.join(cmd)}", file=sys.stderr)
    subprocess.run(cmd, cwd=REPO_ROOT, check=True)


def parse_lcov_zero_hit_lines(text: str) -> dict[str, set[int]]:
    """Map LCOV SF paths to line numbers with DA:<line>,0."""
    by_source: dict[str, set[int]] = defaultdict(set)
    current_sf: str | None = None

    for raw in text.splitlines():
        line = raw.strip()
        if line.startswith("SF:"):
            current_sf = line[3:]
        elif line.startswith("DA:") and current_sf is not None:
            payload = line[3:]
            parts = payload.split(",")
            if len(parts) < 2:
                continue
            try:
                line_no = int(parts[0])
                count = int(parts[1])
            except ValueError:
                continue
            if count == 0:
                by_source[current_sf].add(line_no)
        elif line == "end_of_record":
            current_sf = None

    return dict(by_source)


def resolve_lib_src_path(sf_path: str) -> Path | None:
    """Resolve an LCOV SF path to a file under lib/src."""
    candidate = Path(sf_path)
    if candidate.is_file():
        try:
            return candidate.resolve().relative_to(LIB_SRC.resolve())
        except ValueError:
            pass

    normalized = sf_path.replace("\\", "/")
    marker = "/lib/src/"
    idx = normalized.find(marker)
    if idx != -1:
        rel = normalized[idx + len(marker) :]
        path = LIB_SRC / rel
        if path.is_file():
            return Path(rel)

    if normalized.startswith("lib/src/"):
        path = REPO_ROOT / normalized
        if path.is_file():
            return Path(normalized.removeprefix("lib/src/"))

    # Last resort: match by filename when paths differ (e.g. remap prefixes).
    name = Path(normalized).name
    if not name.endswith(".rs"):
        return None
    matches = list(LIB_SRC.rglob(name))
    if len(matches) == 1:
        return matches[0].relative_to(LIB_SRC)

    return None


UNCOVERED_MARKER = "// 0"


def mark_source(source: Path, uncovered_lines: set[int]) -> str:
    text = source.read_text(encoding="utf-8")
    out: list[str] = []
    for line_no, line in enumerate(text.splitlines(), start=1):
        if line_no in uncovered_lines:
            stripped = line.rstrip()
            out.append(
                UNCOVERED_MARKER
                if not stripped
                else f"{stripped} {UNCOVERED_MARKER}"
            )
        else:
            out.append(line)
    return "\n".join(out) + ("\n" if text.endswith("\n") else "")


def write_marked_sources(
    lcov_by_sf: dict[str, set[int]],
    parsed_dir: Path,
) -> list[tuple[Path, int]]:
    """Write marked files; return (relative lib/src path, uncovered count)."""
    parsed_dir.mkdir(parents=True, exist_ok=True)
    written: dict[Path, set[int]] = {}

    for sf_path, zero_lines in lcov_by_sf.items():
        if not zero_lines:
            continue
        rel = resolve_lib_src_path(sf_path)
        if rel is None:
            continue
        source = LIB_SRC / rel
        if not source.is_file():
            continue
        written.setdefault(rel, set()).update(zero_lines)

    results: list[tuple[Path, int]] = []
    for rel, zero_lines in sorted(written.items(), key=lambda item: str(item[0])):
        marked = mark_source(LIB_SRC / rel, zero_lines)
        out_path = parsed_dir / rel
        out_path.parent.mkdir(parents=True, exist_ok=True)
        out_path.write_text(marked, encoding="utf-8")
        results.append((rel, len(zero_lines)))

    return results


def write_manifest(parsed_dir: Path, results: list[tuple[Path, int]], lcov_path: Path) -> None:
    lines = [
        "# Uncovered executable lines (DA:<line>,0) from LCOV",
        f"# lcov: {lcov_path}",
        "# Format: <lib/src path> <uncovered line count>",
        "",
    ]
    total = 0
    for rel, count in results:
        lines.append(f"{rel} {count}")
        total += count
    lines.extend(["", f"# files: {len(results)}", f"# uncovered lines: {total}"])
    (parsed_dir / "manifest.txt").write_text("\n".join(lines) + "\n", encoding="utf-8")


def build_arg_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument(
        "--skip-run",
        action="store_true",
        help="Do not run coverage; only parse an existing LCOV file",
    )
    parser.add_argument(
        "--lcov",
        type=Path,
        default=DEFAULT_LCOV,
        help=f"LCOV tracefile path (default: {DEFAULT_LCOV.relative_to(REPO_ROOT)})",
    )
    parser.add_argument(
        "--parsed-dir",
        type=Path,
        default=DEFAULT_PARSED,
        help=f"Output directory for marked sources (default: {DEFAULT_PARSED.relative_to(REPO_ROOT)})",
    )
    parser.add_argument(
        "--ignore-filename-regex",
        default=DEFAULT_IGNORE_REGEX,
        help="Forwarded to cargo llvm-cov --ignore-filename-regex",
    )
    return parser


def main() -> int:
    args = build_arg_parser().parse_args()
    lcov_path: Path = args.lcov if args.lcov.is_absolute() else REPO_ROOT / args.lcov
    parsed_dir: Path = (
        args.parsed_dir if args.parsed_dir.is_absolute() else REPO_ROOT / args.parsed_dir
    )

    if not args.skip_run:
        try:
            run_test_coverage_lcov(lcov_path, args.ignore_filename_regex)
        except subprocess.CalledProcessError as err:
            print(f"coverage failed (exit {err.returncode})", file=sys.stderr)
            return err.returncode or 1

    if not lcov_path.is_file():
        print(f"LCOV file not found: {lcov_path}", file=sys.stderr)
        return 1

    lcov_text = lcov_path.read_text(encoding="utf-8", errors="replace")
    lcov_by_sf = parse_lcov_zero_hit_lines(lcov_text)
    if not lcov_by_sf:
        print("No DA:<line>,0 entries found in LCOV (nothing to mark).", file=sys.stderr)
        return 0

    results = write_marked_sources(lcov_by_sf, parsed_dir)
    write_manifest(parsed_dir, results, lcov_path)

    print(f"Wrote {len(results)} marked file(s) under {parsed_dir}", file=sys.stderr)
    for rel, count in results:
        print(f"  {rel}: {count} uncovered line(s)", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
