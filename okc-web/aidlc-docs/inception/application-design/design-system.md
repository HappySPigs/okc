okc-web 디자인 시스템 + 정보구조(IA)·내비게이션 (해커톤 데모 v1 FINAL)

Linear + Vercel/Geist 톤의 "조용한 베이스 + 정확히 3개 focal wow 화면" 전략. 목표는 데모 심사에서 **고유 가치(충돌 리뷰 · Provenance 검증 · 라이브 통합)가 3초 안에 읽히는** 것. 모든 토큰/규칙/라우트는 okc-core 하드 제약(승자 선택 없음, Major/Critical waive 불가, ≤10 소스, freeze-then-run, okc-mcp 이연, 읽기전용 Compiled Vault)을 UI로 정직하게 드러내도록 설계된다.

Stack: Next.js(App Router) + Tailwind + shadcn/ui + Tremor + Lucide + Geist/Inter + Radix Colors.

> 본 문서는 **IA/내비게이션 정본**을 포함한다(§7~§13). 화면별 상세는 `ui-screens.md`. 라우트·아이콘·용어·심각도 표기는 본 문서를 **유일 기준**으로 삼는다(critique 정합성 이슈 해소).

---

# PART A — 비주얼 디자인 시스템

## 1. 색상 토큰 (Radix Colors)

원칙: **중립 스케일 1개 + 브랜드 액센트 1개.** 그 외 색은 "장식"이 아니라 **의미(semantic)** 전용이며 화면당 아주 적게 등장. 데모는 프로젝터 가독성을 위해 라이트 테마 기본, 다크는 Radix dark 스케일 교체 옵션.

### 1.1 팔레트 선택
- 중립(Neutral): Radix `slate`(1~12) — 쿨 그레이, Linear/Geist 톤.
- 액센트(Brand/Interactive): Radix `indigo` — 링크·프라이머리 버튼·포커스 링·활성 탭·현재 스텝. **"행동 유도"에만.**
- 의미(Semantic): 상태 배지 전용. 액센트와 역할이 겹치지 않게 엄격 분리.
  - 성공/승인/Verified: `grass`
  - 경고/Minor/무효화(stale): `amber`
  - 위험/Major·Critical/차단: `red`
  - 모순(Contradiction, 정보성·승자 없음): `violet` — **예약 색.** 오직 모순 표면에만 등장시켜 "특별하고 에러가 아님"을 각인.

Radix 12-step 의미(공통): 1~2 배경 · 3~5 컴포넌트 배경(기본/hover/active) · 6~8 보더(subtle/ui/hover) · 9 solid · 10 solid hover · 11 저대비 텍스트 · 12 고대비 텍스트.

### 1.2 시맨틱 토큰 (CSS 변수 · shadcn 연동)

```css
:root {
  /* Surface */
  --bg-app:        var(--slate-1);
  --bg-subtle:     var(--slate-2);
  --surface:       #ffffff;
  --border-subtle: var(--slate-6);
  --border:        var(--slate-7);
  --border-hover:  var(--slate-8);

  /* Text */
  --text:          var(--slate-12);
  --text-muted:    var(--slate-11);
  --text-faint:    var(--slate-10);

  /* Accent (indigo) */
  --accent-bg:     var(--indigo-3);
  --accent-line:   var(--indigo-7);
  --accent-solid:  var(--indigo-9);
  --accent-hover:  var(--indigo-10);
  --accent-text:   var(--indigo-11);
  --focus-ring:    var(--indigo-8);

  /* Semantic (badge 전용) */
  --danger-bg: var(--red-3);   --danger-line: var(--red-7);   --danger-text: var(--red-11);   --danger-solid: var(--red-9);
  --warn-bg:   var(--amber-3); --warn-line:   var(--amber-7); --warn-text:   var(--amber-11);
  --ok-bg:     var(--grass-3); --ok-line:     var(--grass-7); --ok-text:     var(--grass-11); --ok-solid: var(--grass-9);
  --contra-bg: var(--violet-3);--contra-line: var(--violet-7);--contra-text: var(--violet-11);

  --radius: 0.5rem;
}
```
- shadcn 매핑: `--primary`→`--accent-solid`, `--primary-foreground`→white, `--ring`→`--focus-ring`, `--destructive`→`--danger-solid`, `--muted-foreground`→`--text-muted`.
- 다크: Radix `slateDark`/`indigoDark`/… 스케일로 동일 변수만 교체(스텝 번호 유효). 데모 기본 라이트.
- **색만으로 의미 전달 금지** — 모든 상태는 색 + 아이콘 + 텍스트 3중 인코딩(프로젝터/색약 대비).

## 2. 타이포그래피 (Geist / Inter)

- UI 서체: **Geist Sans**(폴백 Inter), `next/font` self-host. 모노: **Geist Mono** — 경로/토큰/해시/엔드포인트/에러코드/`curator_id` 전용. 표 숫자 `tabular-nums`. 본문 기본 **14px**(Linear식).

| 토큰 | px/lh | weight | tracking | 용도 |
|---|---|---|---|---|
| display | 30/36 | 600 | -0.02em | 로그인·wow 헤로 |
| h1 | 24/32 | 600 | -0.015em | 페이지 제목 |
| h2 | 20/28 | 600 | -0.01em | 섹션 |
| h3 | 16/24 | 600 | 0 | 카드 제목 |
| body | 14/20 | 400 | 0 | 본문·표 |
| body-strong | 14/20 | 500 | 0 | 강조·활성 라벨 |
| label | 13/18 | 500 | 0 | 폼 라벨·배지 |
| caption | 12/16 | 400 | 0 | 메타·타임스탬프(`--text-muted`) |
| mono | 13/20 | 400 | 0 | 해시/경로/토큰/코드 |

규칙: 헤딩=`--text`, 메타/보조=`--text-muted`. 한 화면에 굵기 3개(400/500/600) 이내. 대문자 남발 금지.

## 3. 스페이싱 · 라디우스 · 엘리베이션

- 스페이싱: **4px 그리드**. 리듬 `4·8·12·16·24·32·48`. 페이지 거터 24~32, 카드 패딩 16~20, 폼 필드 간격 12~16, 아이콘-텍스트 gap 8, 배지 pad 6/2. **밀도 있는 표는 촘촘하게, 감싸는 캔버스는 넉넉하게.**
- 라디우스: `--radius=8px`. 카드/패널 12(rounded-xl), 버튼/인풋 8(rounded-lg), 배지/칩 6(rounded-md), 아바타/도트 full.
- 엘리베이션: **보더 우선, 그림자 최소.** 카드=`border+surface`(플랫). 그림자는 떠 있는 요소만 — Popover/Dropdown/Command/Dialog/Sheet/Toast(`shadow-lg`). glass/글로우/그라디언트 금지.
- 보더: 기본 1px `--border`. focus 2px `--focus-ring` + `outline-offset:2px`(항상 visible).

## 4. 컴포넌트 라이브러리 매핑

역할이 겹치면 하나로 통일 — 배지=§6 규격, 사이드 상세=Sheet, 파괴적 확인=AlertDialog.

### 4.1 shadcn/ui — 구조·상호작용
| 영역 | 프리미티브 | 표준 용도 |
|---|---|---|
| 레이아웃/내비 | `Sidebar`,`Breadcrumb`,`Separator`,`ScrollArea` | **RBAC 내비 가시성**: contributor에겐 앱 셸 자체가 없음 |
| 데이터 | `Table`(+TanStack `DataTable`),`Card`,`Badge`,`Avatar`,`Accordion`,`Collapsible`,`HoverCard`,`Tooltip` | 소스/토큰/파일/finding 목록 |
| 입력/폼 | `Form`(rhf+zod),`Input`,`Textarea`,`Select`,`Checkbox`,`Switch`,`RadioGroup`,`Label`,`Button` | waive/omission **사유 필수**=zod `min(1)` 게이팅 |
| 오버레이 | `Dialog`,`AlertDialog`,`Sheet`,`Popover`,`DropdownMenu`,`Command`(⌘K),`Tabs`,`ResizablePanelGroup` | AlertDialog=regenerate/무효화 등 되돌릴 수 없는 확인; Sheet=finding/Provenance 상세; Resizable=E4-3/E5-1 3-패널 |
| 피드백 | `Sonner`,`Skeleton`,`Progress`,`Alert` | Alert 배너=`PROJECT_BUSY` 재시도·승인 무효화 경고 |

### 4.2 Tremor — 대시보드·통계·진행
| 프리미티브 | 표준 용도 |
|---|---|
| `Card`+`Metric`+`Text`+`Flex`+`Grid` | 스탯 타일: 소스 `3/10`, 차단 findings 수, 승인 완료 수 |
| `Tracker` | **IntegrationCheckpoint 파이프라인 단계 전용(블록)** — §13 참조 |
| `ProgressBar`/`ProgressCircle` | **Job 실행 진행률 전용** — §13 참조 |
| `BarList` | 클러스터별/심각도별 finding 분포 |
| `CategoryBar` | 심각도 구성(Critical/Major/Minor) 한 줄 요약, 소스 사용률 |
| `Callout` | 인라인 제약 안내(provenance 면책, okc-mcp 이연, no-clobber) |
| `List` | Provenance 파일 리스트, 매니페스트 파일 목록 |

(차트류는 대시보드 국한, 남용 금지.)

### 4.3 아이콘 · 용어 사전 (SINGLE SOURCE OF TRUTH)

**이 표가 모든 문서의 유일 기준이다.** 아래와 다른 아이콘/용어를 쓰지 않는다(critique fix #3·#10 확정).

**개념 → Lucide 아이콘**
| 개념 | 아이콘(정본) | 비고 |
|---|---|---|
| admin 역할 | `ShieldCheck` | (구 `Shield` 폐기) |
| contributor 역할 | `UserCog`(또는 `User`) | (구 `UploadCloud` 폐기 — 업로드 동작과 구분) |
| 로그인/토큰 | `KeyRound` | |
| **freeze(소스 고정)** | `Lock` | freeze 전용 |
| **blocked(컴파일 차단)** | `Ban` | blocked 전용 (구 `Lock` 중복 사용 해소) |
| 업로드 동작 | `Upload` · `FileArchive`(zip/tar.zst) | |
| 복사/회전/폐기/만료 | `Copy` · `RefreshCw` · `Trash2` · `Clock` | |
| 부서/개인 라벨 | `Building2`(부서) · `UserCog`(개인) | owner_display_name 라벨 |
| 소스/프로젝트/클러스터 | `Files` · `FolderGit2` · `Layers` | |
| 통합/실행/스텝 | `Workflow` · `Play` · `Loader2`(스핀) · `CircleCheck`(완료) · `CircleDot`(현재) · `CircleDashed`(대기) | |
| **Review 내비** | `ScanSearch` | (구 `GitCompare` 폐기) |
| **Critical** | `OctagonAlert` | red solid |
| **Major** | `TriangleAlert` | red soft |
| **Minor** | `CircleAlert` | amber soft |
| waive | `Undo2` | |
| omission | `EyeOff` | |
| regenerate | `RefreshCw` / `Sparkles` | |
| approve | `CircleCheck` / `Check` | |
| 모순(양측·승자 없음) | `GitCompareArrows` / `Scale` | violet |
| **compile 동작** | `Hammer` | (구 `GitMerge` 폐기) |
| Compiled Vault 내비/산출 | `Package`(nav) · `PackageCheck`(성공) | |
| no-clobber 충돌 | `FileWarning` | |
| provenance/검증 | `BadgeCheck`(verified) · `Fingerprint`/`Hash`(해시) · `FileCheck` · `ScrollText`(감사) · `Waypoints`/`Route`(계보) | |
| stale/무효화 | `History` | |
| 서빙/mcp | `Radio`(서빙 nav) · `Server` · `Link2`/`ExternalLink` · `Code2`/`FileJson`(계약) · `PlugZap`(연동) | |
| 공통 | `Search` · `Settings` · `ChevronRight` · `Info` · `LayoutDashboard` · `Users` · `LogOut` · `CircleUser` | |

**용어(정본)**
- 병합 산출물 = **"Compiled Vault"**(사이드바·화면 통일; "병합 Vault 브라우저" 등 혼용 금지, 라우트 `/compiled`).
- 서빙 read API 경로 = **`/api/serving/*`**(구 `/vault/*` 폐기).
- 컴파일 실행 = **"컴파일 실행"**(트리거는 `/compiled` 한 곳).
- 파이프라인 상태 = IntegrationCheckpoint 7단계(§13).

## 5. (통합됨 → §6)

## 6. 심각도 · 상태 배지 규약 (제품의 핵심 언어)

단일 `Badge` + `variant`. 기본형 **soft**(bg=step-3, text=step-11, border=step-6, 20~22px, 13px/500, 선행 아이콘 12px, radius 6px). **solid**(step-9 + 흰 글자)는 화면당 "가장 중요한 단 하나"에만. 배지는 색+아이콘+텍스트 3중 인코딩 필수.

| 배지 | 의미 | 색 | 채움 | 아이콘(정본) | UI가 강제하는 행동 |
|---|---|---|---|---|---|
| **Critical** | 차단·**waive 불가** | red | solid | `OctagonAlert` | 컴파일 CTA disabled. regenerate로만 해소 |
| **Major** | 차단·**waive 불가** | red | soft | `TriangleAlert` | 동일: 컴파일 차단, regenerate만 해소 |
| **Minor** | **waive 가능(사유 필수)** | amber | soft | `CircleAlert` | Waive 버튼 활성 → zod 검증 → Waived |
| **Waived** | 사유와 함께 보류 | slate | outline(dim) | `Undo2` | HoverCard로 사유·작성자. 차단 카운트 제외 |
| **Contradiction** | **모순 보존·승자 없음** | violet | soft | `GitCompareArrows` | 정보성. **"승자 선택" 액션 절대 없음.** 양측 근거 병렬 |
| **Blocked** | 컴파일 차단 상태 | red | soft | `Ban` | Compile disabled + 차단 개수 + "이동" 링크 |
| **Stale·승인 무효화** | freeze 후 소스/설정 변경 | amber | soft(dashed) | `History` | 배너 "이전 승인 무효화" + 재승인 요구 |
| **Approved** | 클러스터 승인됨 | grass | soft | `CircleCheck` | — |
| **Verified** | compile+verify 통과 | grass | solid | `BadgeCheck` | 서빙 공개 가능 |
| **Pending/In review** | 검토 대기 | slate | soft | `CircleDashed` | — |
| **Running/Busy** | Job 진행 / `PROJECT_BUSY` | indigo | soft + spin `Loader2` | `Loader2` | BUSY는 재시도 배너(Alert) |

**절대 규칙(심사 포인트):**
- Major/Critical 행의 Waive/Omit 컨트롤은 **렌더는 하되 disabled + `Tooltip` "Major/Critical은 waive 불가 — regenerate로만 해소".** 존재하지 않는 척하지 않고 제약을 "보이게".
- 모순에는 Approve/Reject/승자 버튼을 **절대** 두지 않는다. 오직 양측 근거 + `Scale` + "보존됨(no winner)".
- 컴파일 버튼은 blocking(Major/Critical) 카운트가 0이 아닌 한 항상 disabled + 사유 배너. 시도 시 `APPROVAL_REQUIRED` 계열을 **코드 기준** 배지화(문자열 파싱 금지).
- **freeze=`Lock`, blocked=`Ban`** — 두 개념에 같은 아이콘을 쓰지 않는다.

---

# PART B — 정보구조(IA) · 내비게이션 · App Shell (정본)

## 7. 개념 계층 (Mental Model)

단일 조직(Q1: 단일 테넌시) → 워크스페이스 하나. 모든 작업은 **프로젝트** 단위이며 프로젝트 내부는 okc-core 상태기계 순서를 그대로 IA로 반영한다.

```
Organization (단일, 암묵적)
└─ Project  (freeze-then-run 단위, 소스 ≤10)
   ├─ Overview        상태·진행·다음 액션                (E3-3)
   ├─ Sources         업로드 착지분 / freeze             (E2·E3-4)
   ├─ Upload Tokens   토큰 발급·조회·폐기                (E2-1/2)
   ├─ Integration     오케스트레이션·실행·진행 모니터    (E3-5)  ★focal
   ├─ Review          Gate / Taxonomy / Cluster          (E4)    ★focal(E4-3)
   ├─ Compiled Vault  컴파일 실행 + 병합 산출 트리        (E5-1)
   └─ Serving         개요·publish / Provenance / 계약    (E5)    ★focal(E5-2)
```

핵심 원칙: **왼쪽 위→아래 사이드바 순서 = 데모 진행 순서 = okc-core 파이프라인 순서.** 사이드바만 봐도 "업로드→고정→통합→리뷰→컴파일→서빙"이 읽힌다.

## 8. 3-Tier 라우트 구조 + 라우트 정본 맵

| Tier | 성격 | 셸 | 예시 |
|---|---|---|---|
| **Global** | 인증·프로젝트 선택·계정/사용자 | App Shell(프로젝트 컨텍스트 없음) | `/login`, `/projects`, `/settings/*` |
| **Project** | 파이프라인 작업 전부 | App Shell(프로젝트 컨텍스트 활성) | `/projects/[id]/*` |
| **Token Upload** | 로그인 없이 토큰만 | **Upload Shell(셸 없음)** | `/u/[token]` |

FR-UP-2를 존중해 contributor 동선은 앱 셸과 **물리적으로 분리**. okc-mcp(비인간)는 UI가 없고 계약은 admin 화면(`/serving/contract`)에 노출.

### 8.1 라우트 정본 맵 (single source of truth)
IA 라우트맵을 정본으로 채택하고 모든 에픽 라우트를 정렬(critique fix #1).

| 라우트 | 화면 | 셸/역할 | 결정 사항 |
|---|---|---|---|
| `/login` | 로그인 | Auth/전체 | contributor 토큰 탭은 `/u/[token]`로 이동만(앱 세션 없음) |
| `/u/[token]` · `/upload` · `/done` | 토큰 업로드 3화면 | Upload/contributor | `/upload`(구 admin 리다이렉트) 라우트 폐기 |
| `/projects` | 프로젝트 목록 | App(global)/admin | |
| `/projects/new` | 생성 모달 | App(modal)/admin | |
| `/settings/users` · `/settings/account` | 사용자·역할 / 내 계정 | App(global)/admin | Global tier — 계정 메뉴 진입 |
| `/projects/[id]` | Overview | App(project)/admin | **전용 화면**(구 병합 허브에서 분리) |
| `/projects/[id]/sources` | 소스 & Freeze | App(project)/admin | **전용 화면** |
| `/projects/[id]/tokens` · `/tokens/new` | 업로드 토큰 | App(project)/admin | |
| `/projects/[id]/integration` (+`?job=live`) | 통합 오케스트레이션 & 진행 모니터 ★focal | App(project)/admin | **전용 화면**(구 병합 허브에서 분리) |
| `/projects/[id]/review` | 리뷰 게이트 | App(project)/admin | 컴파일 트리거 없음(딥링크만) |
| `/projects/[id]/review/taxonomy` | Taxonomy 승인 | App(project)/admin | |
| `/projects/[id]/review/clusters/[clusterId]` (+`?tab=`,`?contra=`) | 클러스터 리뷰 ★focal | App(project)/admin | **path param**(query param 폐기). 모순=이 화면의 "모순 탭"(별도 라우트 없음) |
| `/projects/[id]/review/clusters/[clusterId]/regenerate` | Regenerate diff | App(project)/admin | base |
| `/projects/[id]/compiled` (+`?note=`) | **Compiled Vault**(컴파일 실행 + 트리) | App(project)/admin | **컴파일 트리거 & 3영역 트리 단일 지점** |
| `/projects/[id]/serving` | 서빙 개요 & publish | App(project)/admin | 서빙 서브탭 루트 |
| `/projects/[id]/serving/verify` (+`?note=`) | **Provenance & Verify** ★focal | App(project)/admin | provenance 정본 위치 |
| `/projects/[id]/serving/contract` | okc-mcp 계약 | App(project)/admin | **별도 라우트**(탭 아님) |
| `/api/serving/tree`·`/note`·`/verify`·`/explain` | read-only API | UI 없음/okc-mcp | 구 `/vault/*` 폐기 |

정본 결정 3줄 요약: **(1) 컴파일 산출물·트리·트리거 = `/compiled` 한 곳. (2) provenance = `/serving/verify`. (3) 모순 = E4-3 "모순 탭"(별도 라우트 없음).**

## 9. App Shell 레이아웃 (admin 전용)

```
┌───────────────────────────────────────────────────────────────────────┐
│ TOPBAR (h-14, sticky, border-b)                                         │
│  [breadcrumb: Project ▸ Review ▸ Cluster] ······ [RunStatePill][⌘K][🔔][◐]│
├──────────────┬────────────────────────────────────────────────────────┤
│ SIDEBAR       │  ⚠ PROJECT_BUSY 재시도 배너 (조건부, sticky, amber)     │
│ (w-60, fixed) │  ⚠ 승인 무효화 배너 (조건부, freeze-then-run, amber)     │
│               │ ┌──────────────────────────────────────────────────┐  │
│ [◇ Project ⌄] │ │  MAIN CONTENT                                      │  │
│               │ │   · 읽기형: max-w-3xl 중앙                         │  │
│ Overview      │ │   · 표/대시보드: 풀폭 + p-6/p-8                    │  │
│ Sources  7/10 │ │   · E4-3/E5-1: 3-패널 ResizablePanelGroup          │  │
│ Upload Tokens │ │   · E5-2: lineage 그래프 히어로                    │  │
│ Integration ● │ │                                                    │  │
│ Review    ⛔3 │ │                                                    │  │
│ Compiled ✓    │ └──────────────────────────────────────────────────┘  │
│ Serving       │                                                        │
│ ───────────── │                                                        │
│ [👤 Account ⌄]│                                                        │
└──────────────┴────────────────────────────────────────────────────────┘
```

### 9.1 존(Zone)
| Zone | 크기 | 구성 | 컴포넌트 |
|---|---|---|---|
| Sidebar | `w-60` 고정, `w-14` 아이콘 모드 | 프로젝트 스위처(상단)·섹션 nav(중단)·계정 메뉴(하단) | `ScrollArea`,`Separator`,`Tooltip`(축소),`Collapsible` |
| Topbar | `h-14` sticky border-b | 좌 Breadcrumb / 우 RunStatePill·⌘K·알림·테마 | `Breadcrumb`,`Button(ghost)`,`Badge`,`Search`/`Bell`/`SunMoon` |
| Banner slot | topbar 아래 조건부 | PROJECT_BUSY·승인 무효화 | `Alert(destructive/warning)`+`Button` |
| Main | 나머지 | 화면별 | 화면별 |

### 9.2 2-Tier 내비 (Global tier ↔ Project tier 분리 — critique 정합성 해소)
Global tier와 Project tier를 **평면 혼합하지 않는다.**
- **프로젝트 미선택(Global 라우트):** 사이드바 = `Projects`(`FolderGit2`)만 활성. 사용자·역할 관리(`/settings/users`)·설정은 **계정 메뉴** 안에.
- **프로젝트 선택(Project 라우트):** 사이드바 = 아래 **프로젝트 스코프 7항목**.

| 항목 | 아이콘 | 라우트 | 우측 배지/상태 |
|---|---|---|---|
| Overview | `LayoutDashboard` | `/projects/[id]` | — |
| Sources | `Files` | `…/sources` | `n/10`(10 도달 amber, 11번째 차단) |
| Upload Tokens | `KeyRound` | `…/tokens` | 활성 토큰 수(secondary) |
| Integration | `Workflow` | `…/integration` | 실행 중 `●` 펄스 dot / 대기 무색 |
| **Review** | `ScanSearch` | `…/review` | 차단 findings `⛔N`(destructive) — 0이면 `✓` |
| Compiled Vault | `Package` | `…/compiled` | 컴파일 완료 `✓`, 미완 dimmed+disabled |
| Serving | `Radio` | `…/serving` | published 시 green dot |

- 활성 항목: 좌측 2px accent bar + `bg-muted` + medium weight.
- **비활성 게이팅:** Compiled/Serving은 선행 상태 미충족이면 `disabled`+`Tooltip`("컴파일 승인 완료 후 활성화"). 클릭 원천 차단.
- Serving은 route-backed 서브탭(개요 `/serving` · Provenance `/serving/verify` · okc-mcp 계약 `/serving/contract`)을 갖고, 셋 모두에서 사이드바 "Serving"이 활성. Review도 동일(게이트/taxonomy/clusters/regenerate).

### 9.3 프로젝트 스위처 (사이드바 헤더)
- `Popover`+`Command`(검색형), `ChevronsUpDown`. 현재 프로젝트명+상태 dot. 목록 각 프로젝트에 `IntegrationCheckpoint` 배지. 하단 `+ 새 프로젝트`(admin 전용; contributor에겐 셸 자체가 없음).

## 10. 역할별 내비게이션 가시성 (RBAC-driven, 정본)

**Contributor는 앱 셸이 없다(critique rbac_issues 해소).** 서버가 내려준 역할로 렌더 필터(숨김 우선, API 403 이중 방어).

| 대상 | Persona | 셸 | 접근 라우트 | 사이드바 | 비고 |
|---|---|---|---|---|---|
| **Administrator/Curator** | 인간, full control | App Shell | Global + Project 전부 | Global tier / Project tier(7항목) | `curator_id`로 매핑(FR-AUTH-4) |
| **Contributor** | 인간, 업로드만 | **Upload Shell(셸 없음)** | **`/u/[token]`만** | **없음** | 모든 `/projects/*`·`/settings/*`는 URL 직타해도 **403**(E1-4). `/upload`·`/me/uploads` 라우트 없음 |
| **okc-mcp consumer** | 비인간, read-only | — | `/api/serving/*`(read), 계약은 admin `…/serving/contract` | — | 채팅/쿼리 UI 없음 — **엔드포인트/계약만** |

> 데모 포인트: admin 로그인 = 7섹션 풀 사이드바. contributor 링크 = 사이드바가 **아예 없는** 단일 업로드 카드. "관리자만 통합(②)"이 화면 구조 자체로 증명된다. contributor는 앱 셸 크롬(탑바/Breadcrumb/⌘K/계정 메뉴)에 **전혀 노출되지 않는다.**

## 11. 셸 변형 (Layout Variants)

| 셸 | 사용 라우트 | 구성 | 핵심 컴포넌트 |
|---|---|---|---|
| **Auth Shell** | `/login` | 중앙 단일 `Card`, 로고 상단, 무채색 배경 | `Card`,`Input`,`Button`,`Label`,`Form`,에러 `Alert` |
| **App Shell** | Global+Project(admin) | §9 사이드바+탑바 | 위 전부 |
| **Upload Shell** | `/u/[token]`,`/upload`,`/done` | 셸 없음. 얇은 org 로고 바 + 중앙 업로드 카드. token 스코프 컨텍스트만(대상 프로젝트/슬롯/만료) | `Card`,dropzone(`Input file`),`Progress`(Tremor `ProgressBar`),`Alert`,`Sonner` |

Upload Shell 3상태: 유효(업로드 폼) · 만료/폐기(차단 카드, FR-UP-2 AC) · 완료(수락 확인 + owner_display_name).

## 12. 글로벌 요소

- **12.1 계정 메뉴(사이드바 하단):** `DropdownMenu`, 트리거 `Avatar`+이름+역할 `Badge`(admin=`ShieldCheck` accent). 항목: 프로필 · **사용자·역할 관리**(admin, `/settings/users`) · 테마 · 로그아웃. Lucide `CircleUser`,`Users`,`Settings`,`LogOut`. admin일 때 `Info` 라인: "이 계정 ID(`curator_id`)가 통합 감사에 기록됩니다. okc-core는 이 라벨을 검증하지 않습니다."(FR-AUTH-4)
- **12.2 Command Palette(⌘K):** `Command`(Dialog). 프로젝트/섹션 점프 + 퀵 액션(새 프로젝트, 토큰 발급, 통합 실행). 데모 화면 전환 장치.
- **12.3 토스트 — Job 이벤트 & 에러(Sonner):** 성공/진행=통합 Job `events` 폴링 토스트. 에러=**`OkcError{code,category}` 분기**(문자열 파싱 금지): `transient`=재시도 액션 토스트, `validation`=인라인 폼 에러, `conflict/approval`=해당 섹션 이동 링크.
- **12.4 PROJECT_BUSY 배너:** topbar 아래 sticky `Alert`(amber) + `Loader2`(spin) + "다른 통합 작업이 진행 중입니다" + [재시도](NFR-CONC-1 직렬화/큐잉). 토스트가 아니라 **배너**(지속 상태 명확).
- **12.5 승인 무효화 배너(freeze-then-run):** 소스/설정/taxonomy 변경으로 하위 승인 stale → Overview·Integration·Review 상단 `Alert`(warning) + `History`: "소스가 변경되어 이전 승인이 무효화되었습니다. 다시 통합을 실행하세요."(C-4/FR-INT-2)
- **12.6 Breadcrumb:** `Project명 ▸ 섹션 ▸ (Cluster #12)`. 마지막 노드 비링크 강조. 프로젝트 노드 클릭 → Overview.
- **12.7 Run-State Pill(탑바 우측):** 현재 `IntegrationCheckpoint`를 `Badge`+dot로. `NeedsProvider`/`NeedsSources`→회색 / `NeedsDisclosure`/`NeedsTaxonomy`/`NeedsClusters`→accent(작업요망) / `ReadyToCompile`·`Verified`→grass / 차단 findings 존재→destructive("차단 3"). 클릭 → Integration.

## 13. 체크포인트 Tracker vs Job Phase 진행 (구분 규칙 — critique 정합성 해소)

같은 Tremor `Tracker` 비주얼이 두 종류 상태를 나타내던 혼동을 제거한다. **두 개는 시각적으로 명확히 다르게 그린다.**

- **파이프라인 단계(macro·상시):** Tremor **`Tracker` 블록** 7칸 = IntegrationCheckpoint `NeedsProvider→NeedsSources→NeedsDisclosure→NeedsTaxonomy→NeedsClusters→ReadyToCompile→Verified`. Overview(E3-3)·Integration(E3-5) 헤더에 상주. 색: 완료=grass·현재=indigo·대기=slate·경고(stale)=amber·차단/에러=red. 라벨 **"파이프라인 단계"**.
- **실행 진행(micro·실행 중에만):** **세로 Steps 리스트**(각 phase `CircleDashed`→`CircleDot`/`Loader2`→`CircleCheck`) + Tremor `ProgressBar`/`ProgressCircle`. phase = preflight→embedding→candidate→synthesis→critic. **블록 Tracker를 쓰지 않는다.** 라벨 **"실행 진행"**. E3-5 진행 모니터 전용.

### 13.1 Provider/Disclosure 게이트 (Tracker의 "유령 스텝" 방지 — critique missing_screens 해소)
`NeedsProvider`·`NeedsDisclosure` 두 체크포인트 단계는 데모에서 상호작용 없이 지나가면 유령처럼 보인다. E3-5 Integration에 **Provider & Disclosure 카드**를 두어 명시한다:
- **NeedsProvider** → 배지 "AI provider: 서버 env-var로 사전 프로비저닝됨(A-2) — 승인 불필요"(`Server`). 자동 충족.
- **NeedsDisclosure** → `allow_remote_provider`=true면 **원격 provider 고지 확인 `Switch`**(`remote_disclosure_confirmed`, `Info`/`ShieldAlert`); 로컬/offline이면 배지 "로컬 provider — 고지 불필요".
- 파이프라인 Tracker의 해당 칸에 `Info` 팝오버로 위 상태를 연결해, 두 단계가 왜 자동 통과되는지 설명한다.

## 14. Focal 전략 ("조용한 베이스 + 정확히 3개 wow", critique fix #6 확정)

폴리시 예산의 ~80%를 아래 **3개**에만 집중. 나머지는 의도적으로 담백.

- **베이스(담백):** 로그인/역할, 프로젝트 목록/Overview, 소스/토큰, 리뷰 게이트(E4-1), taxonomy(E4-2), regenerate diff(E4-4), Compiled Vault 브라우저(E5-1), 서빙 개요/계약(E5-3/4), 설정. 흰/slate-1 서피스·보더 카드·밀도 있는 DataTable·indigo 1개·그림자 없음. 소스 상한 `Files 3/10` 칩 상시, 11번째 disabled+`Tooltip`. contributor는 앱 셸 자체가 없음.
- **Focal #1 — E3-5 통합 진행 모니터:** 실행 진행 Steps pulse + `ProgressBar` 채움 + 라이브 append 이벤트 로그(모노 + code Badge 컬러) + `Sonner` 스트림. 파이프라인 블록 Tracker와 실행 Steps를 시각 구분. "엔진 Job의 관측 창"(채팅/쿼리 아님).
- **Focal #2 — E4-3 클러스터 리뷰 워크스페이스(메인 히어로):** 3-패널 워크벤치(좌 클러스터 리스트 / 중 Tabs[Synthesis·모순·Omission·Taxonomy] / 우 **상시 Findings 인스펙터 레일**). severity 배지 대비, disabled 승인 버튼, "컴파일 거부" 배너, 모순 탭 좌우 split(violet, `Scale`, "승자 없음"). **Findings=우측 고정 레일, 모순=중앙 탭**(레이아웃 상충 해소).
- **Focal #3 — E5-2 Provenance & Verify:** 히어로 lineage 그래프(소스→클러스터→합성→노트) draw-in + Verified 배지 scale-in(160ms) + 결정성 해시 강조 + 모순 split "승자 없음". authenticity 고지 `Callout` 상시.

## 15. 모션 (아주 절제)

- 지속시간 대부분 120~180ms, 패널/Sheet 슬라이드 220~250ms. Easing=ease-out(`cubic-bezier(0.16,1,0.3,1)`) 입장, 퇴장은 더 짧게. **transform/opacity만.**
- 허용: Dialog/Sheet 입장(Radix), Sonner 슬라이드, Skeleton shimmer, Progress/Tracker 채움, `Loader2` 스핀, Accordion 펼침, 탭 인디케이터 슬라이드, **E5-2 lineage 그래프 draw-in**.
- 정적 유지: 테이블 행·배지·내비·카드. 스크롤 연동/패럴랙스/바운스 금지.
- 1회성 강조: Blocking 배너 등장 attention pulse(선택), Verified 배지 scale-in. **컨페티/과한 축하 금지.**
- `prefers-reduced-motion` 존중 → 비필수 모션 off. 로딩=`Skeleton`, 스피너는 "액션/Job"에만.

## 16. Do / Don't (데모 폴리시 체크리스트)

**Do**
- 액센트 indigo 하나. 의미색(red/amber/grass/violet)은 "뜻"이 있을 때만.
- 배지 soft 통일, solid는 화면당 히어로 상태 1개.
- 토큰/경로/해시/엔드포인트는 mono + Copy.
- 소스 상한 `3/10` 칩 상시, 11번째 disabled + Tooltip.
- 차단을 "보이게": Compile disabled + red 배너 + 차단 개수 + finding로 점프.
- 에러는 `OkcError{code,category}` 기준 배지 분기(문자열 파싱 금지). `PROJECT_BUSY`=재시도 배너.
- 아이콘/용어/심각도는 **§4.3·§6 사전만** 사용(`OctagonAlert`/`TriangleAlert`/`CircleAlert`, freeze=`Lock`/blocked=`Ban`, compile=`Hammer`, Review=`ScanSearch`, "Compiled Vault", `/api/serving/*`).
- 라우트는 §8.1 정본 맵만 사용(클러스터 리뷰=path param, 모순=E4-3 탭, 컴파일 트리거/트리=`/compiled`).
- 빈 상태엔 명확한 CTA와 ⌘K 힌트. 테이블 밀도 유지, 숫자 tabular-nums·우측 정렬.

**Don't**
- 그라디언트/글로우/글래스/카드 그림자 남발 금지(보더 우선).
- 모순에 "승자 선택" 버튼 두지 말 것 — 절대.
- Major/Critical에 Waive 제공 금지(disabled + "regenerate로만 해소" Tooltip).
- verify FAIL 안내에 'waive' 용어 쓰지 말 것 — **"재컴파일로만 해소"만**(verify=내부 일관성, critic waive와 별개).
- provenance를 스팬 단위로 암시하지 말 것 — **파일 단위 보증(NFR-DET-1)** 고지를 리뷰(E4-3)·검증(E5-2) 양쪽에.
- 서빙 화면에 채팅/쿼리 UI 만들지 말 것(retrieval 이연) — 계약만.
- **contributor에게 앱 셸/프로젝트 라우트 노출 금지** — 셸 자체가 없고 모든 프로젝트 라우트는 403.
- 컴파일 트리거를 두 곳에 두지 말 것(`/compiled` 한 곳; 다른 화면은 딥링크). compiled 3영역 트리도 `/compiled`에만 렌더.
- 파이프라인 체크포인트 블록 Tracker와 Job 실행 진행 Steps를 같은 비주얼로 그리지 말 것(§13).
- 모든 것에 애니메이션·컨페티·바운스 이징 금지. 색만으로 의미 전달 금지(아이콘+텍스트 병기).

---

관련 근거 파일(절대경로):
- `c:/Users/genie/workplace/okc-web/aidlc-docs/inception/requirements/requirements.md`
- `c:/Users/genie/workplace/okc-web/aidlc-docs/inception/requirements/okc-core-capability-analysis.md`