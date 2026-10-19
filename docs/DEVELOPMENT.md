# 개발

## 문서

| 문서 | 내용 |
|---|---|
| [PROTOCOL.md](PROTOCOL.md) | PC ↔ 폰 약속 (UUID·광고·QR·패킷·제어 메시지·확인·잠그기·시험 값) |
| [CODECS.md](CODECS.md) | 코덱 번호 표, 바꾸는 법·추가하는 법 |
| [FEATURES.md](FEATURES.md) | PC·폰 앱의 기능과 그 값 |
| [HISTORY.md](HISTORY.md) | 시행착오. 평소엔 읽지 않고, 그 절의 코드를 고칠 때만 |
| 이 문서 | 구조와 코드 규칙 |
| `../CLAUDE.md` | 결정 기록 ("왜") |

## 구조

| 폴더 | 내용 |
|---|---|
| `src-tauri/src/` | PC 앱 (Rust). 캡처 `capture/`, 코덱·패킷 `codec/`, 인코딩 `encoding.rs`, 연결 확인·잠그기 `auth/`, 폰 한 대 처리 `listener.rs`, BLE 송신 `ble.rs`(Linux)·`ble_windows.rs`(Windows), QR 링크 `link.rs` |
| `src-tauri/examples/` | 화면 없이 송신 (FEATURES.md "공유") |
| `src/` | PC 앱 화면 (React). 배치 `layouts/`, 부품 `components/` |
| `android/` | 폰 앱 (Kotlin) |

## 코드 규칙

- **한 파일 = 한 가지 기능.** 새 기능은 기존 파일에 끼워 넣지 말고 파일을 따로 만든다. 파일이 두 가지 일을 하기 시작하면 그때 쪼갠다.
- 계층끼리 서로 모르게: 캡처 ↔ 코덱 ↔ 전송(BLE)은 바이트/샘플만 주고받는다. (예: `ble.rs`는 코덱을 모름)
- 플랫폼별 구현은 같은 인터페이스의 파일로 나눈다 (`capture/linux`·`windows`, `ble.rs`·`ble_windows.rs` 방식).
- 확인·소리 보내기 순서는 OS와 무관한 `listener.rs` 한 곳에. OS 파일은 그 폰에 보내는 방법(`PhoneLink`)만.

## 주석 규칙

- 값·규칙·기능 설명은 `docs/`의 문서 한 곳에만 적는다. 코드는 그 값을 이름 붙은 상수로 쓴다.
- 주석은 **그 코드가 구현하는 문서와 절 이름만** 가리킨다. 예: `// docs/PROTOCOL.md: 연결 확인`. 절 번호는 쓰지 않는다 (절이 늘거나 순서가 바뀌면 틀어짐).
- 시행착오·함정은 코드에 남기지 않고 [HISTORY.md](HISTORY.md)의 같은 이름의 절에 적는다.
- 값을 바꾸면 문서와 상수를 같이 바꾼다. PC ↔ 폰 약속은 양쪽 코드도 같이.
- 외부 템플릿이 넣은 주석(예: `main.rs`의 Tauri 주석)은 그대로 둔다.

## 기록 규칙

- "무엇이/언제" → Stop hook이 자동 기록. "왜" → `CLAUDE.md` "확정된 결정"에 한 줄 append.
- 사실과 결정만 적는다. "나중에/후보/필요해지면" 같은 정하지 않은 미래 작업은 적지 않는다 (다음 세션이 할 일로 오해함).
