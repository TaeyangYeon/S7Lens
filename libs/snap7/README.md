# snap7 Native Library

Place the prebuilt snap7 binary for your platform in this directory.

## macOS

File: `libsnap7.dylib`

Sources:
- snap7 SourceForge: https://sourceforge.net/projects/snap7/
- Homebrew: `brew install snap7` then copy from `/usr/local/lib/libsnap7.dylib` (Intel Mac)

## Windows

Files: `snap7.dll` + `snap7.lib`

Source: snap7 SourceForge release archives contain prebuilt Windows binaries.

## Linux

File: `libsnap7.so`

Source: snap7 SourceForge, or build from source with `cmake`.

## Development Without a Real PLC

During development without a real PLC or without the library file present, use
`PlcClient::new_mock()`. Mock mode never calls into the native library; unit
tests always use mock mode so the build succeeds even when `libsnap7.dylib` is
absent from this directory.

The `build.rs` emits link flags unconditionally, but the linker only resolves
them at link time when a non-test binary is being built against the native lib.
Run tests with `cargo test` safely without the dylib present.
