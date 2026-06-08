# Siemens PLC Monitor — CLAUDE.md
## Project
Rust + egui desktop app for Siemens S7 PLC DB monitoring.
Dev machine: Intel Mac (x86_64) / macOS.
## Workflow
1. Follow PLAN.md step instructions exactly
2. Implement all files listed in the step
3. Satisfy all verification points
4. Update PROGRESS.md: check off the step and record results/issues
## Coding rules
- Rust 2021 Edition
- Doc-comment all pub functions
- No unwrap() — use Result/Option with explicit handling
- Thread sharing: Arc<Mutex<>> or Arc<RwLock<>>
- snap7 FFI: minimize unsafe blocks, isolate behind PlcClient safe wrapper
## Environment notes
- Dev machine: Intel Mac x86_64 (NOT Apple Silicon)
- snap7 binary: libs/snap7/libsnap7.dylib (linked via build.rs)
- Without PLC: use PlcClient::new_mock()
- Siemens S7 is big-endian — always use from_be_bytes
- Packaging: cargo-bundle → .app, libsnap7.dylib goes in .app/Contents/Frameworks/
