#!/usr/bin/env bash
# macOS .app + .dmg packaging script for Siemens PLC Monitor
#
# Prerequisites:
#   cargo install cargo-bundle
#   libs/snap7/libsnap7.dylib must exist (see libs/snap7/README.md)
#
# NOTE: Replace assets/icon.png with a real 512x512 ICNS-compatible PNG before
# distributing. The current placeholder is a 16x16 solid-colour image.
set -euo pipefail

APP=target/release/bundle/osx/SiemensPLCMonitor.app
DYLIB=libs/snap7/libsnap7.dylib

# Step 1: Build the .app bundle
cargo bundle --release

# Step 2: Copy snap7 dylib into the bundle's Frameworks directory
mkdir -p "$APP/Contents/Frameworks"
cp "$DYLIB" "$APP/Contents/Frameworks/"

# Step 3: Add @rpath so the binary can find the dylib at runtime
install_name_tool \
    -add_rpath @executable_path/../Frameworks \
    "$APP/Contents/MacOS/SiemensPLCMonitor"

# Step 4: Create the distributable DMG
mkdir -p dist
hdiutil create \
    -volname SiemensPLCMonitor \
    -srcfolder "$APP" \
    -ov -format UDZO \
    dist/SiemensPLCMonitor.dmg

echo "Done: dist/SiemensPLCMonitor.dmg"
