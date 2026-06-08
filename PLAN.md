# Siemens PLC Monitor
## Master Development Plan v1.0

Siemens S7 PLC 실시간 DB 모니터링 + C# 클래스 Export 데스크톱 앱.
머신비전 엔지니어를 위한 Melsoft PLC Monitor Utility 대응 툴.
Rust + egui. Intel Mac (x86_64) 개발 + macOS .app/.dmg 패키징. PCRO 워크플로우. 5 Steps / 5 Phases.

---

# Part 1. 프로젝트 개요

## 1.1 프로젝트 정보

| 항목 | 내용 |
|------|------|
| 개발 머신 | Intel Mac (x86_64) / macOS |
| 배포 타겟 | macOS `.app` 번들 + `.dmg` (개발/배포), 크로스 플랫폼 소스 호환 유지 |
| 언어 | Rust (2021 Edition) |
| GUI | egui + eframe |
| PLC 통신 | snap7 C 라이브러리 FFI 바인딩 (`libsnap7.dylib`) |
| 직렬화 | serde + serde_json (설정 파일 저장/불러오기) |
| 폴링 | std::thread + Arc<Mutex<>> (백그라운드 폴링) |
| C# 코드 생성 | 문자열 템플릿 (snap7dotnet ReadClass 호환) |
| 패키징 | `cargo-bundle` (macOS `.app` + `.dmg`) |
| 개발 방식 | PCRO 프롬프트 → Claude Code 구현 → Taeyang 직접 검증 → Git 커밋 |
| 총 개발 단계 | 5 Steps / 5 Phases |

## 1.2 프로젝트 목적

Siemens S7 PLC의 DB(Data Block)를 실시간으로 읽어 **Bool(bit) / Byte / Word / DWord / Int / DInt / Real / String** 타입별로 값을 화면에 표시하는 모니터링 툴.
기존 C# snap7dotnet `ReadClass` 패턴과 호환되는 클래스 코드를 Export하는 기능 포함.

**핵심 기능**
- IP / Rack / Slot / DB Number 입력으로 S7 PLC 연결
- 변수 이름·타입·오프셋을 UI에서 직접 행 추가/삭제하여 정의
- 폴링 주기마다 DB raw bytes를 통째로 읽어 각 변수 파싱
- Bool 값은 깜빡임(blink) 애니메이션으로 ON/OFF 시각화
- String 타입은 연속 Byte를 ASCII 문자열로 조립하여 표시
- 설정(변수 정의 목록) JSON 저장/불러오기
- C# snap7dotnet ReadClass 호환 클래스 코드 Export (Byte 분해 방식 + helper property)

**지원 변수 타입**

| Rust 타입 | Siemens 타입 | 크기 | C# 타입 |
|-----------|------------|------|---------|
| bool | Bool | 1 bit | bool |
| u8 | Byte | 1 byte | byte |
| u16 | Word | 2 bytes | ushort |
| i16 | Int | 2 bytes | short |
| u32 | DWord | 4 bytes | uint |
| i32 | DInt | 4 bytes | int |
| f32 | Real | 4 bytes | float |
| String | Byte 배열 | N bytes | byte×N + string helper |

## 1.3 설계 원칙

1. **단일 바이너리**: egui + eframe으로 외부 런타임 의존 없이 실행
2. **raw bytes 기반 파싱**: snap7로 DB 전체를 byte 배열로 읽고, 오프셋 기반으로 각 변수 파싱
3. **String 타입 통합 표시**: 연속 Byte(PLC_SERIAL_NO_1~30 등)를 하나의 String 변수로 정의, 모니터에서는 문자열로, Export에서는 byte 분해 + helper property로 출력
4. **설정 영속성**: 변수 정의 목록을 JSON으로 저장하여 재시작 후에도 유지
5. **백그라운드 폴링**: UI 스레드 블로킹 없이 별도 스레드에서 PLC 통신

---

# Part 2. 시스템 설계

## 2.1 아키텍처

```
┌─────────────────────────────────────────────────────────┐
│                  eframe (OS 네이티브 윈도우)              │
│  ┌──────────────────────────────────────────────────┐   │
│  │  App State (Arc<Mutex<SharedState>>)             │   │
│  │  ┌──────────────┐  ┌──────────────────────────┐  │   │
│  │  │ Connection   │  │  VarDef 목록              │  │   │
│  │  │ IP/Rack/Slot │  │  (이름/타입/오프셋/길이)  │  │   │
│  │  │ DB Number    │  └──────────────────────────┘  │   │
│  │  └──────────────┘                                │   │
│  │  ┌──────────────────────────────────────────────┐│   │
│  │  │  LiveValue 목록 (폴링 결과)                  ││   │
│  │  │  Bool → blink 상태 포함                      ││   │
│  │  └──────────────────────────────────────────────┘│   │
│  └──────────────────────────────────────────────────┘   │
│                        ↕ Arc<Mutex<>>                    │
│  ┌──────────────────────────────────────────────────┐   │
│  │  Poller Thread (std::thread)                     │   │
│  │  loop { snap7::read_area(DB) → parse → update }  │   │
│  └──────────────────────────────────────────────────┘   │
│                        ↕ FFI                             │
│  ┌──────────────────────────────────────────────────┐   │
│  │  snap7 C 라이브러리 (.so / .dll / .dylib)         │   │
│  └──────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
```

## 2.2 핵심 데이터 모델

```rust
// 변수 타입
pub enum VarType {
    Bool,
    Byte,
    Word,    // u16
    Int,     // i16
    DWord,   // u32
    DInt,    // i32
    Real,    // f32
    String { length: u32 },  // byte 배열 → ASCII 문자열
}

// 변수 정의 (사용자가 UI에서 입력)
pub struct VarDef {
    pub name: String,
    pub var_type: VarType,
    pub byte_offset: u32,
    pub bit_offset: u8,   // Bool일 때만 유효 (0~7)
}

// 실시간 값 (폴링 결과)
pub enum VarValue {
    Bool { value: bool, blink_on: bool },  // blink_on: 깜빡임 표시 상태
    Byte(u8),
    Word(u16),
    Int(i16),
    DWord(u32),
    DInt(i32),
    Real(f32),
    StringVal(String),
    Unknown,
}

// 연결 설정
pub struct ConnectionConfig {
    pub ip: String,
    pub rack: u16,
    pub slot: u16,
    pub db_number: u32,
    pub poll_interval_ms: u64,
}
```

## 2.3 UI 레이아웃

```
┌─────────────────────────────────────────────────────────────┐
│  🔌 Connection                                              │
│  IP: [192.168.0.1]  Rack: [0]  Slot: [1]  DB: [100]        │
│  [Connect]  [Disconnect]    ● Connected  (또는 ✗ Error msg)│
├─────────────────────────────────────────────────────────────┤
│  📋 Variable Definitions                    [+ Add Row]     │
│  ┌──────────────┬────────┬──────┬─────┬────────┬─────────┐  │
│  │ Name         │ Type   │ Byte │ Bit │ Length │         │  │
│  │ Run_Signal   │ Bool   │  0   │  0  │   -    │  [✕]    │  │
│  │ ErrorCode    │ Word   │  2   │  -  │   -    │  [✕]    │  │
│  │ Position     │ DWord  │  4   │  -  │   -    │  [✕]    │  │
│  │ Serial_No    │ String │  8   │  -  │  30    │  [✕]    │  │
│  │ Temperature  │ Real   │  40  │  -  │   -    │  [✕]    │  │
│  └──────────────┴────────┴──────┴─────┴────────┴─────────┘  │
├─────────────────────────────────────────────────────────────┤
│  📡 Live Monitor              Poll: [100]ms  [▶ Start] [■]  │
│  ┌──────────────┬────────┬────────────────────────────────┐  │
│  │ Name         │ Type   │ Value                          │  │
│  │ Run_Signal   │ Bool   │ ●TRUE  (녹색, 깜빡)            │  │
│  │ ErrorCode    │ Word   │ 0x0042  (66)                   │  │
│  │ Position     │ DWord  │ 123456                         │  │
│  │ Serial_No    │ String │ "AB1234567890"                 │  │
│  │ Temperature  │ Real   │ 25.340                         │  │
│  └──────────────┴────────┴────────────────────────────────┘  │
├─────────────────────────────────────────────────────────────┤
│  [💾 Save Config]  [📂 Load Config]  [📤 Export C# Class]  │
└─────────────────────────────────────────────────────────────┘
```

## 2.4 C# Export 출력 형식

```csharp
// Auto-generated by Siemens PLC Monitor
// DB: 100 | Generated: 2025-01-15 10:30:00
// Compatible with snap7dotnet ReadClass<T>

public class DB100
{
    // --- Bool ---
    public bool Run_Signal { get; set; }        // Byte 0, Bit 0

    // --- Word ---
    public ushort ErrorCode { get; set; }       // Byte 2

    // --- DWord ---
    public uint Position { get; set; }          // Byte 4

    // --- String (byte 분해 방식, PLC와 1:1 매핑) ---
    public byte Serial_No_1 { get; set; }       // Byte 8
    public byte Serial_No_2 { get; set; }
    // ... ~ Serial_No_30

    // --- String helper property ---
    public string Serial_No =>
        System.Text.Encoding.ASCII.GetString(new byte[] {
            Serial_No_1, Serial_No_2, /* ... */ Serial_No_30
        }).TrimEnd('\0');

    // --- Real ---
    public float Temperature { get; set; }      // Byte 40
}
```

---

# Part 3. 개발 단계 (5 Steps)

## Phase 1: 프로젝트 초기화 + 데이터 모델

### Step 1 — Cargo 프로젝트 초기화 + 핵심 데이터 모델 + snap7 FFI 골격
- **작업 내용**:
  - `cargo new siemens-plc-monitor` 초기화
  - `Cargo.toml`: egui/eframe, serde/serde_json, snap7 FFI (build.rs + bindgen 또는 수동 extern)
  - `src/model/variable.rs`: `VarType`, `VarDef`, `VarValue` 정의 (serde Serialize/Deserialize 포함)
  - `src/model/session.rs`: `ConnectionConfig` 정의
  - `src/plc/client.rs`: snap7 `Cli_Create`, `Cli_ConnectTo`, `Cli_ReadArea`, `Cli_Destroy` FFI extern 선언 + 안전 래퍼 (`PlcClient` struct)
  - `src/plc/mod.rs`, `src/model/mod.rs` 모듈 정리
  - `build.rs`: snap7 라이브러리 링킹 설정 (플랫폼별 분기: .so/.dll/.dylib)
  - `libs/snap7/` 디렉토리: 플랫폼별 prebuilt 바이너리 placeholder + README 안내
- **생성 파일**: `Cargo.toml`, `build.rs`, `src/main.rs`, `src/model/mod.rs`, `src/model/variable.rs`, `src/model/session.rs`, `src/plc/mod.rs`, `src/plc/client.rs`, `libs/snap7/README.md`
- **검증 포인트**: `cargo build` 성공, FFI 함수 링킹 오류 없음, `VarDef` / `VarValue` 단위 테스트 통과 (`cargo test`)

---

## Phase 2: 백그라운드 폴링 스레드

### Step 2 — 폴링 스레드 + 파싱 엔진
- **작업 내용**:
  - `src/plc/parser.rs`: raw `&[u8]` + `VarDef` → `VarValue` 파싱 함수
    - Bool: `bytes[byte_offset] >> bit_offset & 1`
    - Word/DWord/Int/DInt: big-endian (`u16::from_be_bytes` 등)
    - Real: `f32::from_be_bytes`
    - String: `bytes[byte_offset..byte_offset+length]` → ASCII 트림
  - `src/plc/poller.rs`: `Arc<Mutex<SharedState>>` 기반 폴링 스레드
    - `PlcClient::read_db(db_no, size)` 호출 → raw bytes
    - `VarDef` 목록 순회하며 파싱 → `LiveVar { def, value, last_updated }` 갱신
    - Bool 값 변화 감지 → `blink_timer` 리셋
    - 연결 끊김 감지 → 자동 재연결 시도 (최대 3회)
  - `src/state.rs`: `SharedState` 정의 (`ConnectionConfig`, `Vec<VarDef>`, `Vec<LiveVar>`, `ConnectionStatus`)
- **생성 파일**: `src/plc/parser.rs`, `src/plc/poller.rs`, `src/state.rs`
- **검증 포인트**: 파서 단위 테스트 (각 타입별 known byte → expected value), 폴링 스레드 mock 테스트 (PLC 없이 더미 bytes로 파싱 검증)

---

## Phase 3: egui UI — Connection + Variable Definition

### Step 3 — egui 앱 뼈대 + Connection 패널 + Variable Definition 패널
- **작업 내용**:
  - `src/app.rs`: `App` struct + `eframe::App` trait 구현
    - `Arc<Mutex<SharedState>>` 소유
    - `update()` 루프: Connection 패널 → VarDef 패널 → Monitor 패널 → 버튼 행
  - **Connection 패널**:
    - IP / Rack / Slot / DB Number 텍스트 입력
    - Connect / Disconnect 버튼 → 폴링 스레드 시작/중지
    - 연결 상태 표시 (● Connected / ✗ Error)
  - **Variable Definition 패널**:
    - 테이블 형태: Name / Type(ComboBox) / Byte Offset / Bit Offset / Length(String일 때만) / 삭제 버튼
    - `[+ Add Row]` 버튼으로 빈 행 추가
    - Type ComboBox: Bool / Byte / Word / Int / DWord / DInt / Real / String
    - Bit Offset 셀: Bool 타입일 때만 활성화, 나머지 `-` 표시
    - Length 셀: String 타입일 때만 활성화, 나머지 `-` 표시
- **생성 파일**: `src/app.rs`, `src/main.rs` (eframe::run_native 진입점)
- **검증 포인트**: `cargo run` 으로 윈도우 열림, 행 추가/삭제, ComboBox 타입 변경, Bit/Length 셀 활성화 조건 동작

---

## Phase 4: Live Monitor 패널 + 설정 저장/불러오기

### Step 4 — Live Monitor 실시간 표시 + blink 애니메이션 + 설정 JSON
- **작업 내용**:
  - **Live Monitor 패널**:
    - `SharedState.live_vars` 읽어 테이블 표시
    - Bool: `blink_on` 상태에 따라 녹색(●TRUE) / 회색(○FALSE) 색상 토글, `ctx.request_repaint_after(Duration::from_millis(500))`으로 깜빡임
    - Byte: decimal
    - Word / DWord: `0x{:04X} ({})` 형식 (hex + decimal 병렬 표시)
    - Int / DInt: signed decimal
    - Real: 소수점 3자리 (`{:.3}`)
    - String: `"..."` 따옴표 포함 표시
    - 연결 끊김 시 모든 값 `--` 표시
  - **폴링 주기 설정**: Poll 입력 박스 (ms 단위), ▶ Start / ■ Stop 버튼
  - **설정 저장/불러오기** (`src/config.rs`):
    - `ConfigFile { connection: ConnectionConfig, vars: Vec<VarDef> }` serde 구조체
    - `[💾 Save Config]`: 파일 다이얼로그 → JSON 저장 (`rfd` crate 또는 경로 직접 입력)
    - `[📂 Load Config]`: JSON 불러오기 → VarDef 목록 갱신
- **생성 파일**: `src/config.rs`, `src/app.rs` 확장
- **검증 포인트**: Bool 깜빡임 시각 확인, hex/decimal 동시 표시, JSON 저장 후 재시작해서 불러오기

---

## Phase 5: C# Export + 최종 통합

### Step 5 — C# 클래스 Export + macOS .app/.dmg 패키징 + README
- **작업 내용**:
  - `src/export/csharp.rs`: `VarDef` 목록 → C# 코드 문자열 생성
    - 파일 헤더 (생성 일시, DB 번호, snap7dotnet 호환 안내 주석)
    - Bool → `public bool {name} {{ get; set; }}   // Byte {byte}, Bit {bit}`
    - Byte → `public byte {name} {{ get; set; }}   // Byte {byte}`
    - Word → `public ushort {name} {{ get; set; }}  // Byte {byte}`
    - Int → `public short {name} {{ get; set; }}   // Byte {byte}`
    - DWord → `public uint {name} {{ get; set; }}   // Byte {byte}`
    - DInt → `public int {name} {{ get; set; }}    // Byte {byte}`
    - Real → `public float {name} {{ get; set; }}   // Byte {byte}`
    - String → byte 분해 방식 (`{name}_1` ~ `{name}_{length}`) + helper property
  - `[📤 Export C# Class]` 버튼 → 파일 저장 다이얼로그 (`rfd` crate) → `.cs` 파일 출력
  - **macOS .app 번들 패키징** (`cargo-bundle`):
    - `Cargo.toml`에 `[package.metadata.bundle]` 섹션 추가
      - `name`, `identifier` (예: `com.yourname.siemens-plc-monitor`), `icon`, `version`
    - `cargo bundle --release` → `target/release/bundle/osx/SiemensPLCMonitor.app` 생성
    - `libsnap7.dylib`를 `.app/Contents/Frameworks/` 에 복사하는 `package.sh` 스크립트
    - `install_name_tool -add_rpath @executable_path/../Frameworks ./SiemensPLCMonitor` 으로 rpath 설정
    - `create-dmg` 또는 `hdiutil`로 `.dmg` 생성 스크립트
  - **앱 아이콘**: `assets/icon.png` (512×512) placeholder + `.icns` 변환 안내
  - `README.md`: Intel Mac 개발 환경 설정 (snap7 dylib 준비, `cargo-bundle` 설치), 사용법, 변수 타입 표, Export 예시, `.app` 실행 방법
- **생성 파일**: `src/export/mod.rs`, `src/export/csharp.rs`, `src/app.rs` 확장, `Cargo.toml` bundle 메타데이터, `scripts/package.sh`, `assets/icon.png`, `README.md`
- **검증 포인트**: Export `.cs` 파일 생성 + String byte 분해/helper 정상 출력, `cargo bundle --release` 성공, `.app` 더블클릭으로 실행, `libsnap7.dylib` rpath 오류 없음 (`otool -L` 확인)

---

# Part 3. 개발 규칙

## 3.1 워크플로우

```
1. PLAN.md + PROGRESS.md 첨부
2. "STEP N 진행해줘" 요청
3. Claude가 해당 Step 내용을 보고 아래 3가지를 생성:
   ① PCRO 형식의 Claude Code 프롬프트 (영어)
   ② 직접 검증 방법 (2-Gate)
   ③ Git 커밋 메시지
4. Claude Code에 프롬프트 입력 → 구현
5. Taeyang이 직접 2-Gate 검증 수행
6. 검증 완료 후 Taeyang이 직접 Git 커밋
7. PROGRESS.md Step N 완료 표기 ([ ] → [x]) 및 완료 내역 기록
```

**원칙**
- Claude Code는 Git 커밋하지 않음
- 커밋은 Taeyang이 2-Gate 검증 완료 후 직접 수행
- 하나의 Step = 하나의 커밋

## 3.2 PCRO 프롬프트 규칙

```
## Persona
You are a [구체적인 전문가 역할].

## Context
[프로젝트 배경]
[현재까지 구현된 내용 (이전 Step 결과)]
[이번 Step에서 구현해야 할 내용]

## Restriction
- [하지 말아야 할 것들]
- Do NOT commit to git.

## Output Format
- Implement all files listed below
- After each file, run `cargo build` and confirm it compiles
- Run `cargo test` and confirm all tests PASS
- [생성해야 할 파일 목록 및 경로]
```

**규칙**
1. 프롬프트는 반드시 영어로 작성
2. Persona는 구체적인 전문가 역할 명시
3. Context에 이전 Step 결과물 명시
4. Restriction에 `Do NOT commit to git` 반드시 포함

## 3.3 검증 2-Gate 규칙

매 Step마다 Taeyang이 직접 확인. 2-Gate를 전부 통과해야 다음 Step 진행.

```
Gate 1 — 빌드 + 테스트
  cargo build        → 컴파일 오류 없음 확인
  cargo test         → 전체 GREEN 확인

Gate 2 — 직접 실행 확인
  cargo run          → 앱 실행하여 해당 Step 기능 직접 동작 확인
```

Claude는 Step 내용을 보고 Gate별 구체적인 명령어와 기대 결과를 제공한다.

## 3.4 Git 커밋 규칙

**형식**
```
<type>: <영어 제목>

<한국어 본문>

- 완료 항목 1
- 완료 항목 2
```

**Type**

| type | 용도 |
|------|------|
| feat | 새 기능 추가 |
| fix | 버그 수정 |
| refactor | 리팩토링 |
| docs | 문서 수정 |
| chore | 빌드/설정 변경 |

---

# Part 4. snap7 라이브러리 준비 전략

## 4.1 prebuilt 바이너리 번들 방식 (채택)

개발 머신(Intel Mac x86_64) 기준으로 `libsnap7.dylib`를 `libs/snap7/` 에 포함.
`build.rs`에서 링킹하고, `.app` 번들 패키징 시 `Frameworks/` 안에 포함시켜 배포.

| 플랫폼 | 파일 | 링킹 방식 |
|--------|------|----------|
| **macOS x86_64 (개발 머신)** | `libsnap7.dylib` | `cargo:rustc-link-lib=snap7` |
| Windows x64 | `snap7.dll` + `snap7.lib` | `cargo:rustc-link-lib=snap7` |
| Linux x86_64 | `libsnap7.so` | `cargo:rustc-link-lib=snap7` |

> snap7 공식 배포: https://snap7.sourceforge.net/
> macOS용 prebuilt: `snap7-full-1.4.2/build/osx/` 또는 `brew install snap7`

## 4.2 FFI 바인딩 범위

모니터 앱에 필요한 함수만 최소 바인딩:

```rust
extern "C" {
    fn Cli_Create() -> *mut c_void;
    fn Cli_Destroy(client: *mut *mut c_void) -> i32;
    fn Cli_ConnectTo(client: *mut c_void, address: *const c_char,
                     rack: i32, slot: i32) -> i32;
    fn Cli_Disconnect(client: *mut c_void) -> i32;
    fn Cli_ReadArea(client: *mut c_void, area: i32, db_number: i32,
                    start: i32, amount: i32, word_len: i32,
                    p_usr_data: *mut c_void) -> i32;
    fn Cli_GetConnected(client: *mut c_void, connected: *mut i32) -> i32;
}
```

---

# Part 5. Claude Code 프롬프트 전략

## 5.1 CLAUDE.md (프로젝트 루트에 배치)

```markdown
# Siemens PLC Monitor — CLAUDE.md

## 프로젝트
Rust + egui 기반 Siemens S7 PLC DB 모니터링 데스크톱 앱.
개발 머신: Intel Mac (x86_64) / macOS.

## 워크플로우
1. PLAN.md Step 지시사항 정확히 이행
2. 생성 파일 목록의 모든 파일 구현
3. 검증 포인트 충족 확인
4. PROGRESS.md 해당 Step 체크 및 완료 내역 기록

## 코딩 규칙
- Rust 2021 Edition
- 모든 pub 함수에 주석
- unwrap() 금지 → Result/Option 명시적 처리
- 스레드 간 공유: Arc<Mutex<>> 또는 Arc<RwLock<>>
- egui update()는 60fps 기준으로 최적화 (불필요한 clone 최소화)
- snap7 FFI: unsafe 블록 최소화, PlcClient 안전 래퍼 뒤에 격리

## 환경 주의사항
- 개발 머신: Intel Mac x86_64 (apple silicon 아님)
- snap7 바이너리: libs/snap7/libsnap7.dylib (build.rs 링킹)
- PLC 없는 환경: mock 모드로 더미 bytes 사용
- big-endian 파싱: Siemens S7은 big-endian
- 패키징: cargo-bundle → .app 번들, libsnap7.dylib는 .app/Contents/Frameworks/ 에 포함
```

## 5.2 Step 프롬프트 생성 방식

프롬프트는 매 Step 시작 시 **그 시점의 PLAN.md + PROGRESS.md를 Claude에게 첨부**하여 즉석으로 생성한다.
이전 Step에서 발생한 이슈와 해결 내역이 PROGRESS.md에 기록되어 있으므로, 그 내용이 다음 Step 프롬프트의 Restriction과 Context에 자동으로 반영된다.

**요청 방법**
```
PLAN.md와 PROGRESS.md를 첨부한 뒤:
"STEP N 진행해줘"
```

**Claude가 생성하는 것**
1. 해당 Step의 PCRO 형식 Claude Code 프롬프트 (영어)
2. 2-Gate 검증 방법 및 기대 결과
3. Git 커밋 메시지
