# Postwoman

Rust + egui 기반의 가벼운 API 테스트 데스크탑 앱 (Postman 대체용).

## 다운로드
[Releases](../../releases) 페이지에서 OS에 맞는 zip을 받으세요:
- `Postwoman-macos-arm64.zip` — Apple Silicon Mac (M1 이상)
- `Postwoman-macos-x86_64.zip` — Intel Mac
- `Postwoman-windows-x86_64.zip` — Windows 64bit

> **서명되지 않은 앱 안내**
> - **macOS**: 처음 실행 시 "확인되지 않은 개발자" 경고가 뜨면, 앱을 우클릭 → **열기**를 선택하거나
>   터미널에서 `xattr -dr com.apple.quarantine Postwoman.app` 실행.
> - **Windows**: SmartScreen 경고가 뜨면 **추가 정보 → 실행**.

## 기술 스택
- **언어**: Rust
- **GUI**: `eframe` / `egui` (네이티브 데스크탑)
- **HTTP**: `reqwest` (blocking, rustls-tls) — UI 스레드와 분리된 워커 스레드에서 실행

## 현재 기능 (MVP)
- HTTP 메서드 선택 (GET/POST/PUT/PATCH/DELETE/HEAD/OPTIONS)
- URL 입력 + 쿼리 파라미터 편집 (key/value, 개별 on/off)
- 요청 헤더 편집 (key/value, 개별 on/off)
- 요청 바디 입력 (Raw, JSON 등) — 바디 허용 메서드에서만
- 응답 표시
  - 상태 코드 / 소요 시간(ms) / 응답 크기
  - 본문 (JSON 자동 Pretty-print 토글, 원본 보기)
  - 응답 헤더 목록
- 요청은 비동기 워커 스레드에서 처리되어 전송 중에도 UI가 멈추지 않음

## 실행
```bash
# 개발 빌드 실행
cargo run

# 릴리스 빌드 (최적화 + LTO)
cargo build --release
./target/release/postwoman
```

## macOS 앱 번들
더블클릭으로 실행 가능한 `.app`으로 패키징:
```bash
cargo build --release
./scripts/bundle-macos.sh      # dist/Postwoman.app 생성 (ad-hoc 서명 포함)
open dist/Postwoman.app
```

## 테스트
```bash
cargo test                # 오프라인 단위 테스트
cargo test -- --ignored   # 실제 httpbin 상대 네트워크 통합 테스트
```

## 다음 단계 후보
- 요청 저장 / 컬렉션 (좌측 사이드바)
- 요청 히스토리
- 환경 변수 / `{{변수}}` 치환
- 인증 헬퍼 (Bearer, Basic)
- 응답 본문 검색 / 구문 강조
- 디스크 영속화 (eframe `persistence`)
