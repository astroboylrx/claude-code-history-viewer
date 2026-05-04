<div align="center">

<img src="docs/assets/app-icon.png" alt="CCHV Logo" width="120" />

# Claude Code History Viewer

**[jhlee0409/claude-code-history-viewer](https://github.com/jhlee0409/claude-code-history-viewer) 포크** — 추가 기능 및 Linux 수정 포함.

**Claude Code**, **Codex CLI**, **OpenCode**, **Kimi CLI** 등의 대화 기록을 탐색, 검색, 분석 — 100% 오프라인.

</div>

---

## 추가 기능

- **OpenCode 디렉토리 기반 그룹화** — 단일 "global" 프로젝트 대신 작업 트리별로 세션 그룹화
- **Kimi CLI 지원** — 세션 탐색, 검색, 토큰 통계를 갖춘 완전한 프로바이더
- **통일된 프로젝트 이름** — 모든 프로바이더가 `~/path/to/project` 형식으로 표시
- **프로젝트별 모델 분포** — 개별 프로젝트 통계 페이지의 모델 사용 분석 카드
- **글로벌 통계: 클릭 가능한 인기 프로젝트** — 인기 프로젝트 카드에서 프로젝트를 클릭하여 탐색
- **프로바이더 색상 배지** — 인기 프로젝트 목록의 프로바이더별 배지 (앰버=claude, 그린=codex, 오렌지=kimi, 블루=opencode)
- **폰트 스케일 지원** — 모든 텍스트가 폰트 스케일 슬라이더 (90%-130%)를 따릅니다

<!-- ## Linux / WebKitGTK 수정

- 글로벌 `OverlayScrollbars` 제거 (WebKitGTK 이벤트 처리와 충돌)
- 크기 조정 패널 드래그 후 커서 고정 문제 수정
- 프로젝트 클릭 시 2-4초 멈춤을 피하기 위해 차트 렌더링 지연
- 요소별 Radix Tooltip 트리를 공유 툴팁 시스템으로 교체
- 토큰 분포 차트의 100% 보이지 않는 호 수정 -->

## macOS 설치 (Homebrew)

이 포크는 Apple Developer 인증서가 없어 사전 빌드된 `.dmg` 파일이 Gatekeeper에 의해 차단됩니다. Homebrew를 사용하여 소스에서 빌드하세요 — 앱이 로컬에서 컴파일되어 `/Applications`에 직접 설치됩니다:

```bash
brew install --cask https://raw.githubusercontent.com/astroboylrx/claude-code-history-viewer/main/scripts/claude-code-history-viewer.rb
```

새 릴리즈 후 업데이트:

```bash
brew reinstall --cask https://raw.githubusercontent.com/astroboylrx/claude-code-history-viewer/main/scripts/claude-code-history-viewer.rb
```

## 업스트림

원본 프로젝트는 [jhlee0409/claude-code-history-viewer](https://github.com/jhlee0409/claude-code-history-viewer)를 참조하세요.
