#!/usr/bin/env python3
"""Parse x86_64-w64-mingw32-objdump output from snap7.dll and emit snap7.def."""
import subprocess
import re
import sys
from pathlib import Path

DLL_PATH = Path("libs/snap7/win64/snap7.dll")
DEF_PATH = Path("libs/snap7/win64/snap7.def")

def main():
    result = subprocess.run(
        ["x86_64-w64-mingw32-objdump", "-x", str(DLL_PATH)],
        capture_output=True, text=True
    )
    if result.returncode != 0:
        print(f"objdump failed: {result.stderr}", file=sys.stderr)
        sys.exit(1)

    # Find the [Ordinal/Name Pointer] Table section
    exports = []
    in_table = False
    for line in result.stdout.splitlines():
        if "[Ordinal/Name Pointer] Table" in line:
            in_table = True
            continue
        if in_table:
            # Lines look like:  [   0] +base[   1]  0000 Cli_ABRead
            m = re.match(r'\s*\[\s*\d+\]\s+\+base\[\s*(\d+)\]\s+[0-9a-fA-F]+\s+(\S+)', line)
            if m:
                ordinal = int(m.group(1))
                name = m.group(2)
                exports.append((ordinal, name))
            elif line.strip() == "" and exports:
                # blank line signals end of table
                break

    if not exports:
        print("No exports found — check DLL path or objdump output.", file=sys.stderr)
        sys.exit(1)

    DEF_PATH.parent.mkdir(parents=True, exist_ok=True)
    with DEF_PATH.open("w") as f:
        f.write("LIBRARY snap7\n")
        f.write("EXPORTS\n")
        for ordinal, name in exports:
            f.write(f"    {name} @{ordinal}\n")

    print(f"Written {len(exports)} exports to {DEF_PATH}")

if __name__ == "__main__":
    main()
