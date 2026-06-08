# Siemens PLC Monitor — Progress

## 현재 진행 단계: Step 1 완료

## Phase 1: 프로젝트 초기화 + 데이터 모델
- [x] Step 1: Cargo 초기화 + 핵심 데이터 모델 + snap7 FFI 골격

## Phase 2: 백그라운드 폴링 스레드
- [ ] Step 2: 폴링 스레드 + 파싱 엔진

## Phase 3: egui UI — Connection + Variable Definition
- [ ] Step 3: egui 앱 뼈대 + Connection 패널 + Variable Definition 패널

## Phase 4: Live Monitor + 설정 저장/불러오기
- [ ] Step 4: Live Monitor 실시간 표시 + blink 애니메이션 + 설정 JSON

## Phase 5: C# Export + 최종 통합
- [ ] Step 5: C# 클래스 Export + 빌드 검증 + README

---

<!-- 완료된 Step의 내역은 아래에 역순으로 기록 (최신이 위) -->

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
