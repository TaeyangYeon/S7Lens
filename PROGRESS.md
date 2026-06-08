# Siemens PLC Monitor — Progress

## 현재 진행 단계: Step 4 완료

## Phase 1: 프로젝트 초기화 + 데이터 모델
- [x] Step 1: Cargo 초기화 + 핵심 데이터 모델 + snap7 FFI 골격

## Phase 2: 백그라운드 폴링 스레드
- [x] Step 2: 폴링 스레드 + 파싱 엔진

## Phase 3: egui UI — Connection + Variable Definition
- [x] Step 3: egui 앱 뼈대 + Connection 패널 + Variable Definition 패널

## Phase 4: Live Monitor + 설정 저장/불러오기
- [x] Step 4: Live Monitor 실시간 표시 + blink 애니메이션 + 설정 JSON

## Phase 5: C# Export + 최종 통합
- [ ] Step 5: C# 클래스 Export + 빌드 검증 + README

---

<!-- 완료된 Step의 내역은 아래에 역순으로 기록 (최신이 위) -->

---

## Step 4 완료 기록 — 2026-06-08

### 생성/수정된 파일
| 경로 | 내용 |
|------|------|
| `src/config.rs` | `ConfigFile` 구조체, `save_config`, `load_config`, round-trip 포함 3개 테스트 |
| `src/app.rs` | Live Monitor 패널 (Name/Type/Value 테이블), 폴링 Start/Stop 버튼, Poll ms 입력, Config Save/Load 버튼 + 경로 입력, `format_var_value` 순수 함수, blink repaint 로직, 10개 format 테스트 추가 |
| `src/main.rs` | `mod config;` 등록 |

### 테스트 결과
```
cargo build  → 0 errors, 7 warnings (dead_code: scaffold 단계 정상)
cargo test   → 56 passed, 0 failed
  - app:                 13 tests (기존 3 유지 + format_var_value 10 신규)
  - config:               3 tests (신규: round_trip, missing_file, save_creates)
  - model::variable:     10 tests (Step 1 유지)
  - plc::client:          5 tests (Step 1 유지)
  - plc::mock_data:       5 tests (Step 2 유지)
  - plc::parser:         11 tests (Step 2 유지)
  - plc::poller:          6 tests (Step 2 유지)
  - state:                3 tests (Step 2 유지)
```

### 설계 결정
- **폴링 active 플래그**: `SharedState.polling_active` / `poll_interval_ms` 는 Step 2~3에서 이미 구현 — Step 4에서 ▶ Start / ■ Stop 버튼으로 UI 연결
- **config path 입력 방식**: `rfd` 파일 다이얼로그 대신 `config_path: String` 필드 + `TextEdit` — 의존성 추가 없이 단순하게 처리
- **blink repaint 전략**: `update()` 진입 시 `var_defs`에 Bool 타입이 있으면 `ctx.request_repaint_after(Duration::from_millis(500))` 호출 — 폴러가 500ms 이내로 `blink_on`을 토글하므로 시각적 깜박임 구현
- **format_var_value 분리**: 색상 없는 순수 텍스트 반환 함수로 추출 → 단위 테스트 가능; Bool 색상은 render 함수에서 별도 적용
- **Lock 최소 보유**: Live Monitor 테이블 렌더링 전 `var_defs`, `live_vals`, `status` 를 한 번의 짧은 락으로 클론 → 락 없이 렌더링
- **poll_ms 동기화**: Poll ms 입력 `lost_focus()` 시 `s.poll_interval_ms` 와 `s.config.poll_interval_ms` 모두 업데이트 — 폴러와 Config 저장 양쪽 일치

---

## Step 3 완료 기록 — 2026-06-08

### 생성/수정된 파일
| 경로 | 내용 |
|------|------|
| `Cargo.toml` | `egui_extras = "0.29"` 추가 |
| `src/app.rs` | `PlcMonitorApp`, Connection 패널, Variable Definition 패널, Bottom toolbar, 3개 테스트 |
| `src/main.rs` | `eframe::run_native` 진입점, 900×700 창, poller 스레드 연결 |

### 테스트 결과
```
cargo build  → 0 errors, 6 warnings (dead_code: scaffold 단계 정상)
cargo test   → 43 passed, 0 failed
  - app:                  3 tests (신규: default_inputs, add_remove_var_def, status_display)
  - model::variable:     10 tests (Step 1 유지)
  - plc::client:          5 tests (Step 1 유지)
  - plc::mock_data:       5 tests (Step 2 유지)
  - plc::parser:         11 tests (Step 2 유지)
  - plc::poller:          6 tests (Step 2 유지)
  - state:                3 tests (Step 2 유지)
```

### 설계 결정
- **Draft 입력 분리**: `ConnectionDraft` 구조체를 `PlcMonitorApp`에 보유 — `SharedState` 오염 방지; [Connect] 클릭 시에만 커밋
- **뮤텍스 최소 보유**: 테이블 렌더링 전 `var_defs` 클론 → 변경 수집 → 단일 락으로 적용
- **kind_to_var_type**: String 선택 시 기존 length 보존, 신규 선택 시 기본값 32

### 이슈 해결 — cargo run 즉시 세그멘테이션 폴트

**증상**: `cargo build` 0 에러, `cargo run` → 창 열리지 않고 즉시 SIGSEGV (exit 139)

**원인 1 — egui_extras 이미지 기능**: `egui_extras = "0.29"` 기본 피처에 `image` 크레이트 관련 초기화 코드 포함 → macOS 시스템 라이브러리 없이 실행 시 충돌.
**수정**: `egui_extras = { version = "0.29", default-features = false }` (TableBuilder는 피처 없이 항상 제공됨)

**원인 2 — `Cli_Create()` FFI 호출 (주요 원인)**: `cargo run` (비테스트 빌드) 시 `poller.rs`의 `#[cfg(not(test))]` 분기로 `PlcClient::new()` 호출 → `Cli_Create()` FFI 심볼 런타임 해석 시도. `build.rs`가 `-Wl,-undefined,dynamic_lookup`으로 링크를 허용해 바이너리는 만들어지지만, 실행 시 `libsnap7.dylib` 부재로 심볼 해석 실패 → 즉시 크래시.

**수정**:
- `build.rs`: `snap7_available` 커스텀 cfg 플래그를 dylib 존재 여부로 조건부 발행 (`cargo::rustc-check-cfg=cfg(snap7_available)` 선언 포함)
- `poller.rs`: `#[cfg(any(test, not(snap7_available)))]` → `new_mock()`, `#[cfg(all(not(test), snap7_available))]` → `new()` 로 분기 — dylib 없을 때 FFI 심볼을 절대 호출하지 않음

**수정된 파일**: `build.rs`, `src/plc/poller.rs`, `Cargo.toml`

**검증**: `cargo build` 0 에러 · `cargo run` 창 정상 표시 (6초 생존 확인) · `cargo test` 43 passed
- **status_display 헬퍼**: `ConnectionStatus` → `String` 변환 순수 함수로 분리 → 단위 테스트 가능
- **[Connect] 시 Connecting 상태**: 폴러 스레드가 실제 연결 완료 후 Connected로 전환 (Step 4에서 완성)

---

## Step 2 완료 기록 — 2026-06-08

### 생성된 파일
| 경로 | 내용 |
|------|------|
| `src/plc/parser.rs` | `parse_var` — 8개 VarType 빅엔디언 파싱, 범위 초과 시 `VarValue::Unknown` 반환 |
| `src/plc/mock_data.rs` | `make_mock_db` — 인덱스 mod 256 결정적 테스트 바이트 생성 |
| `src/plc/poller.rs` | `spawn_poller` — 뮤텍스 최소 보유 폴링 루프, blink 로직, 3회 오류 후 재접속 |
| `src/state.rs` | `ConnectionStatus`, `LiveVar`, `SharedState` — 공유 앱 상태 |

### 수정된 파일
| 경로 | 변경 내용 |
|------|----------|
| `src/plc/mod.rs` | `parser`, `poller`, `mock_data` 모듈 추가 |
| `src/main.rs` | `mod state` 추가, `SharedState` 인스턴스화, `spawn_poller` 호출 |

### 테스트 결과
```
cargo build  → 0 errors, 6 warnings (dead_code: scaffold 단계 정상)
cargo test   → 40 passed, 0 failed
  - model::variable:     10 tests (Step 1 유지)
  - plc::client:          5 tests (Step 1 유지)
  - plc::mock_data:       5 tests (새 추가)
  - plc::parser:         11 tests (8개 타입 + 범위 초과 × 2)
  - plc::poller:          6 tests (blink × 3 + compute_read_size × 2 + 통합 × 1)
  - state:                3 tests (new 기본값, LiveVar 생성, ConnectionStatus)
```

### 설계 결정
- **뮤텍스 최소 보유**: config/var_defs를 짧은 락으로 클론 → unlock → FFI 읽기 → 결과 쓰기용 재락
- **blink 로직**: `apply_blink` 헬퍼 분리로 단독 단위 테스트 가능
- **테스트 cfg 격리**: `spawn_poller` 내부에서 `#[cfg(test)]` → `PlcClient::new_mock()`, 비테스트 → `PlcClient::new()` 분기

---

## Step 1 완료 기록 — 2026-06-08

### 생성된 파일
| 경로 | 내용 |
|------|------|
| `Cargo.toml` | edition 2021, egui/eframe 0.29, serde/serde_json |
| `build.rs` | macOS: `-Wl,-undefined,dynamic_lookup` + 조건부 `-lsnap7`; test 빌드 시 링크 플래그 스킵 |
| `libs/snap7/README.md` | 플랫폼별 snap7 바이너리 배치 안내 |
| `src/model/variable.rs` | `VarType`, `VarDef`, `VarValue` + Display/Default/serde |
| `src/model/session.rs` | `ConnectionConfig` + Default/serde |
| `src/model/mod.rs` | 모듈 재-export |
| `src/plc/client.rs` | snap7 FFI extern "C" + `PlcClient` (mock 지원) |
| `src/plc/mod.rs` | 모듈 재-export |
| `src/main.rs` | 최소 스텁 |
| `CLAUDE.md` | 프로젝트 가이드라인 |

### 테스트 결과
```
cargo build  → 0 errors, 14 warnings (dead_code: scaffold 단계 정상)
cargo test   → 15 passed, 0 failed
  - model::variable: 9 tests (serde round-trip × 8 variants + Display + Default)
  - plc::client:     5 tests (mock mode read/connect/disconnect/is_connected)
```

### 해결된 이슈
- **libsnap7.dylib 없이 `cargo build` 성공**: `build.rs`에서 (1) `CARGO_CFG_TEST`를 확인해 테스트 빌드 시 링크 플래그 전체 스킵, (2) dylib 파일 존재 여부로 `-lsnap7` 조건부 발행, (3) macOS에서 `-Wl,-undefined,dynamic_lookup` 발행해 undefined 심볼 허용.
- **`cargo test` 중 FFI 심볼 없음**: `extern "C"` 블록에 `#[cfg(not(test))]` 적용; FFI 호출 경로도 동일 cfg로 분리.
- **VarType String serde**: `#[serde(tag = "kind", content = "length")]`로 인접 태그 직렬화 구현.
