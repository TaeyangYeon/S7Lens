#!/usr/bin/env bash
set -e

# 1. Install Rust Windows target (if not already)
rustup target add x86_64-pc-windows-gnu

# 2. Check mingw-w64 (brew install mingw-w64 if missing)
if ! command -v x86_64-w64-mingw32-gcc &>/dev/null; then
  echo "mingw-w64 not found. Run: brew install mingw-w64"
  exit 1
fi

# 3. Extract snap7 win64 files from archive (if not already extracted)
SNAP7_ARCHIVE="snap7-full-1.4.2.7z"  # place in project root
WIN64_DIR="libs/snap7/win64"
mkdir -p "$WIN64_DIR"

if [ ! -f "$WIN64_DIR/snap7.dll" ]; then
  7z e "$SNAP7_ARCHIVE" "snap7-full-1.4.2/build/bin/win64/snap7.dll" -o"$WIN64_DIR/" -y
fi

# Generate snap7.def from DLL exports (no gendef required)
if [ ! -f "$WIN64_DIR/snap7.def" ]; then
  python3 scripts/gen_def.py
fi

# 4. Generate libsnap7.a from .def (import library for mingw linker)
if [ ! -f "$WIN64_DIR/libsnap7.a" ]; then
  x86_64-w64-mingw32-dlltool -d "$WIN64_DIR/snap7.def" -l "$WIN64_DIR/libsnap7.a"
fi

# 5. Build
cargo build --release --target x86_64-pc-windows-gnu

echo ""
echo "Build complete:"
echo "   EXE: target/x86_64-pc-windows-gnu/release/siemens-plc-monitor.exe"
echo "   DLL: $WIN64_DIR/snap7.dll"
echo "   -> Copy both files to the same folder on Windows"
