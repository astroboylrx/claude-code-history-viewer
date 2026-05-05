<div align="center">

<img src="docs/assets/app-icon.png" alt="CCHV Logo" width="120" />

# Claude Code History Viewer

**[jhlee0409/claude-code-history-viewer](https://github.com/jhlee0409/claude-code-history-viewer) 포크** — 추가 기능 및 Linux 수정 포함.

**Claude Code**, **Codex CLI**, **OpenCode**, **Kimi CLI** 등의 대화 기록을 탐색, 검색, 분석 — 100% 오프라인.

</div>

<div align="center">
<img src="docs/assets/screenshot.png" alt="Screenshot" width="100%" />
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
<!-- - **서브에이전트 세션 필터링** — 모든 통계에서 서브에이전트 세션 제외
- **개선된 활동 히트맵** — 더 큰 타일 (20px), 포털 기반 툴팁, 히트맵 아래로 도구 차트 이동
-->

<!-- ## Linux / WebKitGTK 수정

- 글로벌 `OverlayScrollbars` 제거 (WebKitGTK 이벤트 처리와 충돌)
- 크기 조정 패널 드래그 후 커서 고정 문제 수정
- 프로젝트 클릭 시 2-4초 멈춤을 피하기 위해 차트 렌더링 지연
- 요소별 Radix Tooltip 트리를 공유 툴팁 시스템으로 교체
- 토큰 분포 차트의 100% 보이지 않는 호 수정 -->

## 설치

### Linux

[최신 릴리즈](https://github.com/astroboylrx/claude-code-history-viewer/releases/latest)에서 `.AppImage`를 다운로드하세요:

```bash
chmod +x Claude*.AppImage
./Claude*.AppImage
```

### Windows

[최신 릴리즈](https://github.com/astroboylrx/claude-code-history-viewer/releases/latest)에서 설치 프로그램 (`.exe`)을 다운로드하세요.

### macOS (소스에서 빌드)

이 앱은 유료 Apple Developer 인증서를 사용하지 않아, 사전 빌드된 `.dmg`를 다운로드하면 macOS Gatekeeper가 차단합니다. 이를 우회하려면 몇 단계만으로 로컬에서 앱을 컴파일할 수 있습니다.

**1. 빌드 의존성 설치 (없는 경우)**

`pnpm`과 `rust`가 설치되어 있어야 합니다. Homebrew를 통해 쉽게 설치할 수 있습니다:

```bash
brew install node pnpm rust
```

**2. 소스 코드 다운로드**

```bash
git clone --depth 1 --branch v1.11.1 https://github.com/astroboylrx/claude-code-history-viewer.git
cd claude-code-history-viewer
```

**3. 패키지 설치 및 앱 빌드**

```bash
pnpm install --frozen-lockfile
pnpm tauri build --no-sign
```

빌드 중에 Finder 창이 잠시 나타날 수 있습니다 — 이는 정상이며 몇 초 후 자동으로 닫힙니다.

**4. 앱을 Applications 폴더로 이동**

```bash
cp -r "src-tauri/target/release/bundle/macos/Claude Code History Viewer.app" "/Applications/"
```

**5. 정리 (선택 사항)**

다운로드한 소스 코드 폴더를 삭제하여 공간을 확보할 수 있습니다:

```bash
cd ..
rm -rf claude-code-history-viewer
```

## 업스트림

원본 프로젝트는 [jhlee0409/claude-code-history-viewer](https://github.com/jhlee0409/claude-code-history-viewer)를 참조하세요.
