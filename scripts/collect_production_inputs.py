#!/usr/bin/env python3
"""
collect_production_inputs.py - Aggregator for all production system textual inputs.

This script scans and collects all textual inputs feeding into the BourbakiMesh
production systems (Lean 4 specifications, goal targets, Mathlib CIC ASTs,
worker prompt schemas, curriculum corpora, and configuration manifests)
into a single consolidated file in the project root, delimited by the source filename.
"""

from __future__ import annotations

import argparse
import os
import sys
from pathlib import Path
from typing import List, Tuple

# Default relative paths or globs of textual inputs to the production systems
INPUT_PATTERNS = [
    # 1. Lean 4 Formal Definitions & Exporter Specifications
    "lean_target/lakefile.toml",
    "lean_target/lean-toolchain",
    "lean_target/LeanTarget/**/*.lean",
    "lean_target/*.lean",

    # 2. Open Goal Targets for Distributed Mesh Coordinator & Edge Worker
    "artifacts/target_*.json",

    # 3. Canonical Exported Mathlib / Lean 4 Theorem ASTs
    "artifacts/exported_*.json",

    # 4. Neural Worker Prompts, In-Context DSL Schemas & Edge Controllers
    "ui/src/workers/*.ts",
    "ui/src/services/meshClient.ts",
    "ui/src/services/proofSearchEngine.ts",
    "ui/src/services/llmController.ts",

    # 5. Mathlib & Curriculum Corpora Inputs
    "data/curriculum/curriculum_manifest.json",
    "data/mathlib/corpus.json",
    "data/mathlib_corpus.json",
    "data/mathlib_raw.json",

    # 6. Service & Build Configuration Manifests
    "Cargo.toml",
    "crates/*/Cargo.toml",
    "pyproject.toml",
    "ui/package.json",
    "ui/vite.config.ts",
]

# File extensions to treat as text
TEXT_EXTENSIONS = {
    ".lean",
    ".json",
    ".ts",
    ".tsx",
    ".toml",
    ".md",
    ".txt",
    ".yaml",
    ".yml",
    ".py",
    ".rs",
    "",  # e.g., lean-toolchain
}


def is_text_file(filepath: Path) -> bool:
    """Check if file has a known text extension and can be decoded as UTF-8."""
    if filepath.suffix.lower() not in TEXT_EXTENSIONS and filepath.name != "lean-toolchain":
        return False
    try:
        with open(filepath, "r", encoding="utf-8") as f:
            f.read(2048)
        return True
    except (UnicodeDecodeError, IsADirectoryError, PermissionError):
        return False


def collect_input_files(root_dir: Path) -> List[Path]:
    """Find and deduplicate all matching production input files."""
    matched_files: set[Path] = set()

    for pattern in INPUT_PATTERNS:
        # Handle glob pattern
        for path in root_dir.glob(pattern):
            if path.is_file() and is_text_file(path):
                matched_files.add(path.resolve())

    # Sort files by relative path for deterministic output
    return sorted(list(matched_files), key=lambda p: str(p.relative_to(root_dir)))


def format_delimiter(rel_path: str) -> str:
    """Generate a clear, standard delimiter banner for each source file."""
    banner_line = "=" * 80
    return f"\n{banner_line}\n>>> SOURCE FILE: {rel_path}\n{banner_line}\n"


def aggregate_inputs(root_dir: Path, output_file: Path) -> Tuple[int, int, int]:
    """
    Read all collected textual inputs and write them into the destination file.
    Returns (file_count, total_lines, total_bytes).
    """
    input_files = collect_input_files(root_dir)
    total_lines = 0
    total_bytes = 0

    with open(output_file, "w", encoding="utf-8") as out:
        # Header banner
        header = (
            "================================================================================\n"
            "BOURBAKIMESH PRODUCTION SYSTEMS - CONSOLIDATED TEXTUAL INPUTS BUNDLE\n"
            f"Generated from: {root_dir.name}\n"
            f"Total input sources collected: {len(input_files)}\n"
            "================================================================================\n"
        )
        out.write(header)
        total_lines += header.count("\n")
        total_bytes += len(header.encode("utf-8"))

        for file_path in input_files:
            rel_path = file_path.relative_to(root_dir).as_posix()
            try:
                content = file_path.read_text(encoding="utf-8")
            except Exception as e:
                print(f"[Warning] Failed to read {rel_path}: {e}", file=sys.stderr)
                continue

            delimiter = format_delimiter(rel_path)
            out.write(delimiter)
            out.write(content)
            if not content.endswith("\n"):
                out.write("\n")

            total_lines += delimiter.count("\n") + content.count("\n") + (1 if not content.endswith("\n") else 0)
            total_bytes += len(delimiter.encode("utf-8")) + len(content.encode("utf-8"))

    return len(input_files), total_lines, total_bytes


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Collect all textual inputs to production systems into a single delimited root file."
    )
    parser.add_argument(
        "-o",
        "--output",
        type=str,
        default="production_inputs_bundle.txt",
        help="Name or path of the output text file (default: production_inputs_bundle.txt in project root)",
    )
    parser.add_argument(
        "--root",
        type=str,
        default=None,
        help="Root directory of the project (default: parent directory of this script)",
    )

    args = parser.parse_args()

    # Determine project root
    if args.root:
        root_dir = Path(args.root).resolve()
    else:
        root_dir = Path(__file__).resolve().parent.parent

    # Determine output file path
    output_path = Path(args.output)
    if not output_path.is_absolute():
        output_path = root_dir / output_path

    print(f"📦 Scanning textual production inputs from: {root_dir}")
    file_count, total_lines, total_bytes = aggregate_inputs(root_dir, output_path)

    print(f"✅ Successfully compiled {file_count} input files into: {output_path.name}")
    print(f"   - File size:  {total_bytes / 1024:.2f} KB ({total_bytes:,} bytes)")
    print(f"   - Line count: {total_lines:,} lines")
    print(f"   - Output location: {output_path}")


if __name__ == "__main__":
    main()
