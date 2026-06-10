# S7Lens — Siemens PLC Monitor

A lightweight desktop tool for real-time monitoring of Siemens S7 PLC data blocks (DBs). Define your variables once, watch live values update at your chosen poll rate, then export a ready-to-use C# data class compatible with `snap7dotnet ReadClass<T>`. No PLC config software required.

Built with **Rust + egui** and the open-source [snap7](http://snap7.sourceforge.net/) library. Single binary, no installer.

---

## Features

- **Live DB monitoring** — connects over TCP/IP to any Siemens S7-300/400/1200/1500 PLC and reads a full Data Block at a configurable poll rate (default 100 ms)
- **All S7 primitive types** — Bool (with bit offset), Byte, Word, Int, DWord, DInt, Real, and String
- **Bool blink animation** — TRUE signals pulse green / grey so alarm states are impossible to miss
- **JSON config persistence** — save and reload your variable layout between sessions
- **C# class export** — one click generates a `public class DB{N}` skeleton ready for `snap7dotnet ReadClass<T>`, including byte-decomposed String fields and an ASCII helper property
- **Mock mode** — runs without a real PLC for UI development and testing (`cargo run` on any machine)
- **Cross-platform build** — macOS (native `.app` bundle) and Windows x86_64 (cross-compiled from macOS with mingw-w64)

---

## UI Overview

```
┌─────────────────────────────────────────────────────────────┐
│  🔌 Connection                                              │
│  IP: [192.168.0.1]  Rack: [0]  Slot: [1]  DB: [100]        │
│  [Connect]  [Disconnect]          ● Connected               │
├─────────────────────────────────────────────────────────────┤
│  📋 Variable Definitions                    [+ Add Row]     │
│  ┌──────────────┬────────┬──────┬─────┬────────┬──────────┐ │
│  │ Name         │ Type   │Byte  │ Bit │ Length │          │ │
│  │ Run_Signal   │ Bool   │  0   │  0  │   -    │  [✕]     │ │
│  │ ErrorCode    │ Word   │  2   │  -  │   -    │  [✕]     │ │
│  │ Position     │ DWord  │  4   │  -  │   -    │  [✕]     │ │
│  │ Serial_No    │ String │  8   │  -  │  30    │  [✕]     │ │
│  │ Temperature  │ Real   │  40  │  -  │   -    │  [✕]     │ │
│  └──────────────┴────────┴──────┴─────┴────────┴──────────┘ │
├─────────────────────────────────────────────────────────────┤
│  📊 Live Monitor          Poll: [100] ms  [▶ Start] [■ Stop]│
│  ┌──────────────┬────────┬──────────────────────────────────┐│
│  │ Name         │ Type   │ Value                            ││
│  │ Run_Signal   │ Bool   │ ●TRUE  (green, blinking)         ││
│  │ ErrorCode    │ Word   │ 0x0042  (66)                     ││
│  │ Position     │ DWord  │ 0x0001E240  (123456)             ││
│  │ Serial_No    │ String │ "AB1234567890"                   ││
│  │ Temperature  │ Real   │ 25.340                           ││
│  └──────────────┴────────┴──────────────────────────────────┘│
├─────────────────────────────────────────────────────────────┤
│  Config file: [config.json]  [💾 Save Config] [📂 Load]     │
│  Export file: [output.cs]    [📤 Export C# Class]           │
└─────────────────────────────────────────────────────────────┘
```

---

## Variable Type Reference

| S7 Type    | Rust `VarType`      | C# type   | Size      | Notes                          |
|------------|---------------------|-----------|-----------|--------------------------------|
| BOOL       | `Bool`              | `bool`    | 1 bit     | Bit offset 0–7 within the byte |
| BYTE       | `Byte`              | `byte`    | 1 byte    |                                |
| WORD       | `Word`              | `ushort`  | 2 bytes   | Displayed as hex + decimal     |
| INT        | `Int`               | `short`   | 2 bytes   | Signed                         |
| DWORD      | `DWord`             | `uint`    | 4 bytes   | Displayed as hex + decimal     |
| DINT       | `DInt`              | `int`     | 4 bytes   | Signed                         |
| REAL       | `Real`              | `float`   | 4 bytes   | Displayed to 3 decimal places  |
| STRING[n]  | `String { length }` | `string`  | n bytes   | ASCII, null-trimmed            |

All multi-byte types are parsed as **big-endian** (Siemens S7 standard).

---

## Getting Started

### Run with mock data (no PLC needed)

```bash
cargo run
```

The app opens with simulated zero-valued data. You can define variables, test the export, and save/load configs without a real PLC.

### Run tests

```bash
cargo test
```

---

## Windows Build (cross-compile from macOS)

### Prerequisites

| Tool | Install |
|------|---------|
| Rust stable (2021) | `curl https://sh.rustup.rs -sSf \| sh` |
| mingw-w64 cross-compiler | `brew install mingw-w64` |
| p7zip | `brew install p7zip` |
| `snap7-full-1.4.2.7z` | Download from snap7.sourceforge.net → place in project root |

### Build

```bash
bash scripts/build-windows.sh
```

The script installs the `x86_64-pc-windows-gnu` Rust target, extracts `snap7.dll` from the archive, generates a mingw import library, and produces the release binary.

**Output:**

| File | Location |
|------|----------|
| `siemens-plc-monitor.exe` | `target/x86_64-pc-windows-gnu/release/` |
| `snap7.dll` | `libs/snap7/win64/` |

### Deploy to Windows

Copy both files into the same folder — no installer required:

```
SiemensPLCMonitor\
├── siemens-plc-monitor.exe
└── snap7.dll
```

---

## macOS Build

Place `libsnap7.dylib` (from `snap7-full-1.4.2/build/osx/`) in `libs/snap7/`, then:

```bash
cargo build --release
```

To produce a `.app` bundle:

```bash
cargo install cargo-bundle
cargo bundle --release
bash scripts/package.sh   # copies libsnap7.dylib into .app/Contents/Frameworks/
```

---

## Usage

1. **Connection** — enter the PLC IP, rack, slot, and DB number, then click **Connect**. The status indicator turns green when the connection succeeds.
2. **Variable Definitions** — click **+ Add Row** for each variable you want to watch. Set the name, type, byte offset, and (for Bool) bit offset or (for String) length. Use **✕** to remove a row.
3. **Live Monitor** — click **▶ Start** to begin polling. Values refresh at the poll interval. Bool variables blink green (TRUE) / grey (FALSE). Click **■ Stop** to pause.
4. **Save / Load Config** — type a file path and use **💾 Save Config** or **📂 Load Config** to persist your variable layout as JSON.
5. **Export C# Class** — type a `.cs` output path and click **📤 Export C# Class**. The generated file is compatible with `snap7dotnet ReadClass<T>`.

---

## C# Export Example

Variables defined for DB 100:

| Name        | Type       | Byte | Bit | Length |
|-------------|------------|------|-----|--------|
| Running     | Bool       | 0    | 3   | —      |
| Temperature | Real       | 2    | —   | —      |
| Serial      | String     | 6    | —   | 4      |

Generated output:

```csharp
// Auto-generated by Siemens PLC Monitor
// DB: 100 | Generated: 2026-06-10 09:00:00
// Compatible with snap7dotnet ReadClass<T>

public class DB100 {
    public bool Running { get; set; } // Byte 0, Bit 3
    public float Temperature { get; set; } // Byte 2
    public byte Serial_1 { get; set; } // Byte 6
    public byte Serial_2 { get; set; } // Byte 7
    public byte Serial_3 { get; set; } // Byte 8
    public byte Serial_4 { get; set; } // Byte 9
    public string Serial =>
        System.Text.Encoding.ASCII.GetString(new byte[] { Serial_1, Serial_2, Serial_3, Serial_4 }).TrimEnd('\0');
}
```

String fields are decomposed into individual `byte` properties (one per DB byte) so `snap7dotnet` can map them directly, with a computed `string` property for convenient access.

---

## Architecture

```
UI thread (egui / eframe)
      │
      │  Arc<Mutex<SharedState>>
      │
Poller thread (std::thread)
      │
      │  FFI
      │
snap7 C library (libsnap7.dylib / snap7.dll)
      │
Siemens S7 PLC  ─── TCP/IP (port 102, ISO-on-TCP)
```

The poller thread holds the lock only while reading config and writing results — never during FFI calls or sleeps — keeping the UI responsive at all times.

---

## License

This project uses [snap7](http://snap7.sourceforge.net/) which is licensed under the GNU Lesser General Public License v3.
