#!/usr/bin/env python3
"""Prepare private pinned resources for Claudium's native development client.

The preparation itself is claudium-prepare (crates/claudium-prepare), the Rust port of what this
script did; this wrapper keeps its command line for the development tools that call it. It runs
the claudium-prepare beside this folder (a release folder), else builds it with cargo. Arguments
are passed through: --prepare, --server-jar PATH, --minecraft-dir PATH.
"""
import os
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]


def main():
    arguments = ['--root', str(ROOT), *sys.argv[1:]]
    beside = ROOT / ('claudium-prepare.exe' if os.name == 'nt' else 'claudium-prepare')
    if beside.is_file():
        command = [str(beside), *arguments]
    else:
        command = ['cargo', 'run', '--quiet', '--release', '-p', 'claudium-prepare', '--', *arguments]
    return subprocess.call(command, cwd=ROOT)


if __name__ == '__main__':
    sys.exit(main())
