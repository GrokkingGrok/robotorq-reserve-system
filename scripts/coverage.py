#!/usr/bin/env python3
"""
Coverage wrapper for cargo llvm-cov that adds safe ignore filters and optionally prunes
noisy build-script artifacts under target/llvm-cov-target to keep reports clean.

Usage:
    python scripts/coverage.py [--no-prune] [--ignore-regex REGEX]

Defaults:
    --ignore-regex: excludes rustup, .cargo, and build-script-build objects
    --no-prune: skip deletion of build-script build artifacts

Note: This wrapper runs `cargo llvm-cov` and will rebuild instrumentation. It may take a few
minutes on the first run.
"""

import argparse
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TARGET_LLVM = ROOT / "target" / "llvm-cov-target"
DEFAULT_IGNORE = r"\\\\\.rustup\\\\|\\\\\.cargo\\\\|build_script_build-"  # doubled escapes for Windows


def run(cmd, cwd=ROOT):
    print("Running:", " ".join(cmd))
    proc = subprocess.run(cmd, cwd=cwd, shell=False)
    if proc.returncode != 0:
        raise SystemExit(proc.returncode)


def prune_build_script_artifacts():
    """Remove build-script-build executables under target llvm-cov-target debug/build.
    This removes noisy build-script *exe files that appear in llvm-cov object lists.
    """
    build_dir = TARGET_LLVM / "debug" / "build"
    if not build_dir.exists():
        print("No build-script artifact directory found; skipping prune.")
        return

    removed = []
    for p in build_dir.rglob("*"):
        if p.is_file():
            name = p.name.lower()
            # common noisy names from cargo builds
            if name.startswith("build_script_build-") or name.startswith("build-script-build") or re.search(r"build_script_build-.*\\.exe", name):
                try:
                    p.unlink()
                    removed.append(str(p))
                except Exception as e:
                    print(f"Failed to remove {p}: {e}")
    if removed:
        print(f"Pruned {len(removed)} build-script artifact(s):")
        for r in removed:
            print(" -", r)
    else:
        print("No build-script artifacts found to prune.")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--no-prune", action="store_true", help="Skip pruning build-script artifacts")
    parser.add_argument("--ignore-regex", default=DEFAULT_IGNORE, help="Regex to pass to llvm-cov --ignore-filename-regex")
    parser.add_argument("--extra-args", nargs=argparse.REMAINDER, help="Extra args appended to cargo llvm-cov")
    args = parser.parse_args()

    ignore_flag = ["--ignore-filename-regex", args.ignore_regex]

    cmd = [
        "cargo",
        "llvm-cov",
        "--workspace",
        "--html",
        "--",
        # forward only the ignore regex to llvm-cov
    ]
    # Append ignore regex flag
    cmd.extend(ignore_flag)

    # allow passing additional args (forwarded to llvm-cov)
    if args.extra_args:
        cmd.extend(args.extra_args)

    print("Root:", ROOT)
    print("Target llvm-cov dir:", TARGET_LLVM)
    print("Running coverage with ignore regex:", args.ignore_regex)

    profdata = TARGET_LLVM / "robotorq-reserve-system.profdata"

    # If profdata doesn't exist, run cargo llvm-cov once to produce it (no filtering).
    if not profdata.exists():
        print("profdata not found, running initial 'cargo llvm-cov' to generate profiles...")
        run(["cargo", "llvm-cov", "--workspace", "--html"])

    # Build llvm-cov command to generate HTML with an ignore regex directly
    # Locate llvm-cov executable
    llvm_cov = shutil.which("llvm-cov")
    if not llvm_cov:
        # Try rustup toolchain path
        possible = Path.home() / ".rustup" / "toolchains"
        llvm_cov = None
        if possible.exists():
            for child in possible.iterdir():
                candidate = child / "lib" / "rustlib" / "x86_64-pc-windows-msvc" / "bin" / "llvm-cov.exe"
                if candidate.exists():
                    llvm_cov = str(candidate)
                    break
    if not llvm_cov:
        print("Could not find llvm-cov on PATH or rustup toolchains. Please install llvm-cov or add it to PATH.")
        raise SystemExit(1)

    # Collect object files (executables) under target llvm-cov-target debug
    objs = []
    for d in (TARGET_LLVM / "debug").rglob("*.exe"):
        objs.append(str(d))
    # Also include debug/deps
    deps_dir = TARGET_LLVM / "debug" / "deps"
    if deps_dir.exists():
        for d in deps_dir.rglob("*.exe"):
            objs.append(str(d))

    outdir = ROOT / "target" / "llvm-cov" / "html"
    if outdir.exists():
        shutil.rmtree(outdir)
    outdir.mkdir(parents=True, exist_ok=True)

    cmd2 = [
        llvm_cov,
        "show",
        "-format=html",
        "-instr-profile",
        str(profdata),
        "-output-dir",
        str(outdir),
        "-ignore-filename-regex",
        args.ignore_regex,
    ]
    # Add object flags
    for o in objs:
        cmd2.extend(["-object", o])

    print("Running llvm-cov show to produce filtered HTML report...")
    run(cmd2)

    # optionally prune build-script artifacts after generating report
    if not args.no_prune:
        prune_build_script_artifacts()
    else:
        print("Skipping prune as requested (--no-prune)")

    print(f"Coverage run complete. Report available at {outdir / 'index.html'}")


if __name__ == "__main__":
    main()
