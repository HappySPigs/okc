okc-web — UI 화면 인벤토리 (per-screen inventory, 해커톤 데모 v1 FINAL)

> 본 문서는 화면별 **정본(single source of truth)** 명세다. 라우트·아이콘·용어·상태 배지 규약은 `design-system.md`(IA/Nav 병합본)를 따른다. 모든 라우트는 `design-system.md §8`의 라우트 정본 맵과 1:1로 정렬되어 있으며, 아이콘/심각도 표기는 `design-system.md §5.3(아이콘·용어 사전)`과 `§6(심각도·상태 배지)`를 유일 기준으로 삼는다.

---

## 0. 디자인 방향 요약 (recap)

- 톤: **Linear + Vercel/Geist** — 조용한 베이스(넉넉한 여백, 단일 accent=indigo, 정보밀도는 표/리스트/배지), 그림자 대신 보더.
- 폴리시 예산은 **정확히 3개 focal "wow" 화면**에만 집중한다(critique fix #6로 확정):
  1. **E3-5 통합 오케스트레이션 & 진행 모니터** (Tremor Tracker pulse + 라이브 append 로그)
  2. **E4-3 클러스터 리뷰 워크스페이스** (승인/waive/omission/regenerate 결정 표면 + Major/Critical 차단)
  3. **E5-2 Provenance & Verify 뷰** (소스→클러스터→합성→노트 lineage 그래프 draw-in + verify PASS + 승자 없음 모순 split)
- 나머지 화면(E4-1 리뷰 게이트, E4-2 taxonomy, E4-4 regenerate diff, E5-1/3/4 등)은 **의도적으로 담백한 베이스**로 강등해 대비로 focal을 부각한다.
- okc-core 하드 제약을 **정직하게** UI로 노출: 승자 선택 없음, Major/Critical waive 불가 & 컴파일 차단, ≤10 소스, freeze-then-run 무효화, okc-mcp serving-only(채팅/쿼리 UI 없음), 읽기전용 Compiled Vault, PROJECT_BUSY 재시도 배너, OkcError code/category 분기.

**RBAC 대전제(정본):** contributor는 **앱 셸이 없다.** 유일한 접점은 토큰 업로드 셸 `/u/[token]`. 모든 프로젝트/글로벌 앱 라우트(`/projects/*`, `/settings/*`)는 admin 전용이며 contributor가 직타 시 **403(E1-4)**. 어떤 프로젝트 스코프 화면(E5 포함)도 contributor에게 "읽기 조회"를 허용하지 않는다.

---

## 1. 전역 화면 · 라우트 맵 (Screen / Route Map)

| Screen ID | 화면명 | Route | 페르소나 | Epic | FR | 셸 | FOCAL |
|---|---|---|---|---|---|---|---|
| E1-1 | 로그인 | `/login` | admin(비밀번호) / contributor(토큰→`/u/[token]` 이동만) | E1 | FR-AUTH-1,2 · FR-UP-2 | Auth | — |
| E1-2 | 앱 셸 · RBAC 내비 · 계정 메뉴 | `/(app)` 레이아웃 | **admin 전용** | E1 | FR-AUTH-3,4,2 | App | — |
| E1-3 | 사용자·역할 관리 | `/settings/users` | admin | E1 | FR-AUTH-2,3,4 | App(global) | — |
| E1-4 | 접근 거부(403)·세션 만료(401) | `/403` + 전역 401 인터셉터 | contributor(403)/전체(401) | E1 | FR-AUTH-1,3 | App/전역 | — |
| E1-5 | 내 계정 | `/settings/account` | admin | E1 | FR-AUTH-1 | App(global) | — |
| E2-1 | 업로드 토큰 관리 콘솔 | `/projects/[id]/tokens` | admin | E2 | FR-UP-1,4 | App(project) | — |
| E2-2 | 토큰 발급 다이얼로그(1회성 시크릿) | `/projects/[id]/tokens/new` | admin | E2 | FR-UP-1,2,4 | App(modal) | — |
| E2-3 | 업로드 포털(토큰 랜딩) | `/u/[token]` | contributor | E2 | FR-UP-2 | Upload | — |
| E2-4 | Vault 업로드 & 검증 | `/u/[token]/upload` | contributor | E2 | FR-UP-2,3,4 | Upload | mini-polish |
| E2-5 | 업로드 완료 & 소스 등록 상태 | `/u/[token]/done` | contributor(+admin이 E2-1에서 반영) | E2 | FR-UP-4 | Upload | — |
| E3-1 | 프로젝트 목록 & 통합 대시보드 | `/projects` | admin | E3 | FR-INT-1,2,8 · FR-AUTH-3,4 | App(global) | — |
| E3-2 | 새 프로젝트 생성 모달(curator_id) | `/projects/new` | admin | E3 | FR-INT-1 · FR-AUTH-4 | App(modal) | — |
| E3-3 | 프로젝트 Overview | `/projects/[id]` | admin | E3 | FR-INT-2,8 | App(project) | — |
| E3-4 | 소스 & Freeze | `/projects/[id]/sources` | admin | E3 | FR-INT-1,2 · FR-UP-4 | App(project) | — |
| E3-5 | 통합 오케스트레이션 & 진행 모니터 | `/projects/[id]/integration` (+`?job=live`) | admin | E3 | FR-INT-2,7,8 | App(project) | **★FOCAL** |
| E3-6 | 소스 고정/변경 무효화 경고 | `/projects/[id]/sources` 위 `AlertDialog` | admin | E3 | FR-INT-1,2 | App(dialog) | — |
| E4-1 | 리뷰 개요 & 컴파일 게이트 | `/projects/[id]/review` | admin | E4 | FR-INT-3,4,5,6,7,8 | App(project) | — |
| E4-2 | Taxonomy 편집·승인 | `/projects/[id]/review/taxonomy` | admin | E4 | FR-INT-3,2,8 | App(project) | — |
| E4-3 | 클러스터 리뷰 워크스페이스 | `/projects/[id]/review/clusters/[clusterId]` (+`?tab=`,`?contra=`) | admin | E4 | FR-INT-4,5,6,8 | App(project) | **★FOCAL** |
| E4-4 | Regenerate 피드백 & 재생성 결과 diff | `/projects/[id]/review/clusters/[clusterId]/regenerate` | admin | E4 | FR-INT-5,4,8 | App(project) | — |
| E5-1 | Compiled Vault (컴파일 실행 + 병합 Vault 브라우저) | `/projects/[id]/compiled` (+`?note=`) | admin | E5/E3 | FR-INT-7 · FR-SRV-1 | App(project) | — |
| E5-2 | Provenance & Verify 뷰 | `/projects/[id]/serving/verify` (+`?note=`) | admin | E5 | FR-SRV-2 · FR-INT-6 · NFR-DET-1 | App(project) | **★FOCAL** |
| E5-3 | 서빙 개요 & publish | `/projects/[id]/serving` | admin | E5 | FR-SRV-1 · FR-AUTH-3 | App(project) | — |
| E5-4 | okc-mcp 연동 계약 | `/projects/[id]/serving/contract` | admin | E5 | FR-SRV-3 | App(project) | — |
| — | read-only 서빙 API | `/api/serving/tree`·`/note`·`/verify`·`/explain` | okc-mcp(비인간) | E5 | FR-SRV-1,2 | UI 없음 | — |

정본 결정 요약(critique 반영):
- **컴파일 산출물 & 트리 = `/compiled` 한 곳에만 렌더**(E5-1). 컴파일 **실행 트리거도 여기 한 곳**. E4-1/E3-3/E3-5는 딥링크만.
- **Provenance/Verify = `/serving/verify`**(E5-2). 노트 단위 진입은 `?note=`.
- **okc-mcp 계약 = 별도 라우트 `/serving/contract`**(E5-4). 서빙 섹션은 route-backed 서브탭(개요·Provenance·계약)으로 묶는다.
- **모순 뷰 = E4-3 워크벤치의 "모순" 탭**(별도 라우트 없음). 특정 모순 딥링크는 `?tab=contradictions&contra=<id>`.
- **클러스터 리뷰 = path param** `/clusters/[clusterId]`(query param 폐기).
- **서빙 read API = `/api/serving/*`**(구 `/vault/*` 폐기).

---

## Epic E1 — 인증 & 권한 (FR-AUTH-1..4, NFR-SEC-1)

E1은 데모의 문지기. 비-포컬·극도로 절제. 정직하게 노출할 두 진실: (C-1) okc-core엔 auth가 없어 게이팅 100%가 okc-web 책임 → **역할별 셸 물리 분리**로 증명. (FR-AUTH-4) admin 계정 ID가 `curator_id`(비검증 라벨)로 기록됨을 고지.

### E1-1 — 로그인 `/login`
- **페르소나/목적:** admin(비밀번호 로그인 → 앱 셸 진입), contributor(토큰 붙여넣기 → `/u/[token]`로 **이동만**, 앱 셸 세션 생성 안 함). FR-AUTH-1의 로그인/거부/401.
- **레이아웃:** Auth Shell. 화면 중앙 `max-w-sm` 단일 `Card`, 상단 제품 마크(`KeyRound` + "okc-web"). 카드 상단 `Tabs`[비밀번호 | 업로드 토큰]. footer 고지(caption): "인증·권한은 okc-web이 강제합니다. okc-core는 호출자를 인증하지 않습니다."(C-1)
- **핵심 컴포넌트:** shadcn `Card/CardHeader/CardTitle/CardDescription/CardContent/CardFooter`, `Tabs`, `Form`(rhf+zod), `Label`, `Input`(email/password/token, 비밀번호 우측 `Eye`/`EyeOff` 토글), `Button`(로딩 `Loader2`), `Alert`(destructive/warning), `Separator`, `Sonner`. Lucide: `KeyRound`, `LogIn`, `TriangleAlert`, `ShieldCheck`.
- **인터랙션:** 비밀번호 탭 = email+password → zod 형식검증 → 서버검증 → 성공 시 **admin만** `/projects`로 착지(역할=admin 아니면 로그인 거부). 업로드 토큰 탭 = 토큰 붙여넣기 → 유효 시 **공개 라우트 `/u/[token]`로 네비게이트**(전체 로그인 불필요·앱 셸 세션 없음, FR-UP-2). Enter 제출, 비밀번호 마스킹(NFR-SEC-1, 평문 에코 금지).
- **상태:** idle · validation(zod 인라인) · submitting(`Button` 스핀·입력 disabled) · **error-401**(`Alert(destructive)` "계정 ID 또는 비밀번호가 올바르지 않습니다." — 오류 **코드 기준** 분기) · **session-expired**(E1-4에서 넘어온 상단 `Alert(warning)`) · **account-disabled**(`Alert(destructive)` "비활성화된 계정입니다.") · **token-invalid/expired/revoked**(토큰 탭, `Alert(destructive)`) · success(`Sonner` + 이동).
- **표시 데이터:** 계정 ID, 비밀번호(마스킹), 토큰(에코 안 함), 오류 코드→메시지 매핑.
- **매핑:** FR-AUTH-1, FR-AUTH-2(admin 착지), FR-UP-2(토큰-온리 경계, `/upload` 리다이렉트 없음), NFR-SEC-1 · 원 ①②.
- **FOCAL:** 아니오. 유일한 미세 폴리시: 성공 시 카드 fade + 역할 배지 미리보기.

### E1-2 — 앱 셸 · RBAC 내비 · 계정 메뉴 `/(app)` 레이아웃
- **페르소나/목적:** **admin 전용 셸.** contributor는 이 셸에 진입하지 않는다(셸 자체가 없음). FR-AUTH-3의 RBAC를 **내비 가시성 + 셸 물리 분리**로 구현. 계정 메뉴에서 `curator_id` 고지(FR-AUTH-4).
- **레이아웃:** 좌측 `Sidebar`(w-60, collapsible rail w-14) + 상단 `h-14` sticky topbar(좌 Breadcrumb / 우 Run-State Pill·⌘K·알림·테마) + 배너 슬롯 + 메인 아웃렛. 사이드바 상단 = **프로젝트 스위처**, 중단 = 섹션 nav, 하단 = 계정 카드(`Avatar`+표시명+역할 `Badge`+`ChevronsUpDown`).
- **2-Tier 내비(정본, `design-system.md §9` 상세):**
  - **프로젝트 미선택(Global)**: `/projects`(`FolderGit2`)만 활성. 사용자·역할 관리/설정은 **계정 메뉴** 안에.
  - **프로젝트 선택(Project)**: 7항목 프로젝트 스코프 사이드바 = Overview(`LayoutDashboard`)·Sources(`Files`, `n/10` 배지)·Upload Tokens(`KeyRound`)·Integration(`Workflow`, 실행 중 펄스 dot)·Review(`ScanSearch`, 차단 `⛔N`/`✓`)·Compiled Vault(`Package`, 미완 시 dimmed+disabled)·Serving(`Radio`, published 시 green dot).
  - Global tier와 Project tier를 **평면 혼합하지 않는다.**
- **핵심 컴포넌트:** shadcn `SidebarProvider/Sidebar/SidebarMenu/SidebarMenuItem/SidebarMenuButton/SidebarRail`, `Breadcrumb`, `ScrollArea`, `Separator`, `Tooltip`(rail 라벨), `Collapsible`, `Popover`+`Command`(프로젝트 스위처), `Avatar`, `Badge`(역할: admin=`ShieldCheck` accent solid), `DropdownMenu`(계정 메뉴), `Command`(⌘K), `Skeleton`(세션 해석 중), `Sonner`. Lucide: 위 아이콘 세트 + `CircleUser`, `Users`, `Settings`, `LogOut`, `Info`(curator_id).
- **인터랙션:** 세션 role claim으로 nav 렌더(클라이언트+서버 이중, 서버 가드가 최종 권위). 계정 메뉴 → 표시명/이메일/역할 + `Info` 라인 "이 계정 ID(`curator_id`)가 통합 감사에 기록됩니다. okc-core는 이 라벨을 검증하지 않습니다."(FR-AUTH-4). 로그아웃 → `/login`. 테마/rail 토글.
- **상태:** admin 뷰(전체 nav) · 세션 로딩(nav `Skeleton` — 잘못된 메뉴 깜빡임 방지) · active route 하이라이트(좌측 2px accent bar + `bg-muted`) · collapsed rail · 로그아웃 confirm(선택 `AlertDialog`). 도메인 배너(PROJECT_BUSY·무효화·차단)는 자식 라우트가 배너 슬롯으로 주입.
- **표시 데이터:** 표시명, 이메일, 역할, curator_id 라벨, 현재 경로, 프로젝트 스위처 목록(각 프로젝트 체크포인트 배지).
- **매핑:** FR-AUTH-3(RBAC 내비/셸), FR-AUTH-4, FR-AUTH-2 · 원 ②.
- **FOCAL:** 아니오(데모 결정적 "증명" 화면). 폴리시 절제.

### E1-3 — 사용자·역할 관리 `/settings/users`
- **페르소나/목적:** admin 전용(계정 메뉴 진입, contributor는 nav에 없고 직타 시 403). FR-AUTH-2 역할 부여/변경/회수, FR-AUTH-4 curator_id 매핑 명시.
- **레이아웃:** 타이틀 + 우측 "사용자 추가"(`Button`+`UserPlus`) + 좌측 검색 `Input`(`Search`). 얇은 스탯 행(Tremor `Card`+`Metric` 3: 총 사용자/admin 수/contributor 수). 메인 `DataTable`.
- **핵심 컴포넌트:** shadcn `Table`(+TanStack `DataTable`), `Badge`(역할 admin=`ShieldCheck` solid accent / contributor=`UserCog` neutral, 상태 Active=grass/Inactive=slate), 행 `DropdownMenu`(`MoreHorizontal`: 역할 변경`Pencil`, 활성/비활성 `UserX`), `Dialog`(생성: `Form`+`Input`+`Select` 역할), `Switch`, `AlertDialog`(마지막 admin 가드), `Tooltip`, `Skeleton`, `Sonner`. Tremor `Card`/`Metric`. Lucide `UserPlus`, `Info`, `ShieldCheck`, `UserCog`.
- **컬럼:** 표시명(Avatar+이름) · 계정 ID(mono) · 역할 Badge · 상태 Badge · 마지막 로그인 · curator_id 매핑(admin 행 `Info` 툴팁) · 작업.
- **인터랙션:** 생성(이메일 중복검증, 초대 토큰/임시 비번 **1회만 표시** NFR-SEC-1) · 역할 변경(`Select` 즉시 반영 + `Sonner`) · 활성/비활성(비활성 계정=로그인 거부, E1-1 account-disabled와 연결) · admin 행 `Info` "이 계정 ID가 프로젝트 `curator_id`로 기록됩니다(okc-core 비검증)."
- **상태:** loading(skeleton rows) · empty(첫 사용자 CTA) · populated · 생성 Dialog(idle/validating/submitting/duplicate 오류) · **마지막 admin 가드**(`AlertDialog` 차단 "시스템에 최소 1명의 admin이 필요합니다") · self-demote 경고 · error(`Alert`)/success(`Sonner`).
- **매핑:** FR-AUTH-2,3,4, NFR-SEC-1 · 원 ①②.
- **FOCAL:** 아니오.

### E1-4 — 접근 거부(403) · 세션 만료(401) `/403` + 전역 인터셉터
- **페르소나/목적:** C-1의 정직한 반영. contributor의 admin 라우트 직타 → 403. 미인증/만료 변경성 요청 → 401.
- **레이아웃:** 403 = 중앙 `Card`(`Ban` 큰 아이콘 + "접근 권한이 없습니다" + 설명 + 복구 CTA). 401 = 전역 인터셉터가 코드 감지 → `Sonner` "세션이 만료되었습니다" + `/login` 리다이렉트(E1-1 session-expired 배너 활성). 진행 중 폼 있으면 `Dialog`(세션 만료) 재로그인 유도.
- **핵심 컴포넌트:** shadcn `Card`, `Alert(destructive)`, `Button`, `Dialog`, `Sonner`. Lucide `Ban`(403), `TimerOff`(401), `LogIn`, `ArrowLeft`.
- **인터랙션:** 403 유일 CTA는 로그인/허용 영역 복귀(contributor는 앱 세션 자체가 없으므로 사실상 `/login` 안내). 401은 오류 **코드/카테고리** 분기(문자열 파싱 금지). 메시지 정직: "이 작업은 admin 역할만 수행할 수 있습니다."
- **상태:** 403 forbidden · 401 session-expired · 네트워크 오류(재시도) · 복구. okc-core 도메인 오류(PROJECT_BUSY 등)는 E3/E4 소관 — 이 화면은 okc-web 자체 auth 오류 전담.
- **매핑:** FR-AUTH-1(401), FR-AUTH-3(403) · 원 ②.
- **FOCAL:** 아니오.

### E1-5 — 내 계정 `/settings/account`
- admin이 표시명/비밀번호 변경. shadcn `Form`/`Card`/`Input`/`Button`. 비-포컬. FR-AUTH-1.

---

## Epic E2 — 개인 Vault 업로드 & 토큰 (FR-UP-1..4)

토큰 관리(E2-1/2)는 **admin 전용 프로젝트 스코프**. 업로드 동선(E2-3/4/5)은 **셸 없는 Upload Shell** `/u/[token]`. `owner_display_name`/부서·개인 라벨은 okc-core 비검증 장식 라벨(도움말 "신원 보증 아님"). 프로젝트당 **소스 ≤10**, 11번째 차단. freeze 후 등록 시 "이전 승인 무효화" 경고.

### E2-1 — 업로드 토큰 관리 콘솔 `/projects/[id]/tokens`
- **레이아웃:** Breadcrumb + PageHeader + 우측 "토큰 발급"(`Button`+`Plus`). KPI 행 Tremor `Card` 3(활성 토큰 수 `Metric` / 사용 슬롯 `n/10` `CategoryBar` / 미사용 토큰). 본문 `DataTable` + 검색 `Input` + 상태/라벨 `Select` 필터.
- **핵심 컴포넌트:** shadcn `Table/DataTable`, `Badge`, `DropdownMenu`, `AlertDialog`(폐기), `Tooltip`, `Input`, `Select`, `Skeleton`, `Alert`, `Sonner`. Tremor `Card`/`Metric`/`CategoryBar`. Lucide `KeyRound`,`Plus`,`Copy`,`RefreshCw`(회전),`Trash2`(폐기),`Clock`(만료),`Building2`(부서),`UserCog`(개인),`Ban`.
- **컬럼:** 라벨(부서 `Building2`/개인 `UserCog` + owner_display_name) · 상태 Badge(활성/만료/폐기/미사용) · 슬롯 `#n` · 생성/만료/마지막 사용 · 등록 소스(SourceId 또는 "-") · 액션.
- **인터랙션:** 발급(→E2-2 modal) · 업로드 URL 복사(`Sonner`) · 회전(`RefreshCw`) · 폐기(`AlertDialog`→즉시 업로드 차단) · 검색/필터.
- **상태:** empty(EmptyState+CTA) · loading · **슬롯 10/10**(발급 버튼 disabled + `Tooltip` "소스 상한(10) 도달 — 기존 소스 제거 필요", `CategoryBar` 만석) · 폐기 토큰 dim · **freeze 후(승인 존재)** 경고 배지 "새 업로드는 이전 승인 무효화" · **PROJECT_BUSY** 상단 재시도 배너 · error(`OkcError` code별 `Alert`).
- **매핑:** FR-UP-1, FR-UP-4(≤10), FR-AUTH-3 · 원 ④.
- **FOCAL:** 아니오.

### E2-2 — 토큰 발급 다이얼로그(1회성 시크릿) `/projects/[id]/tokens/new`
- **레이아웃(2-step Dialog):** Step1 폼 = `RadioGroup`(부서/개인) → `Input`(owner_display_name) → `Select`(TTL) → 슬롯 미리보기 배지 `#8 / 10` → 취소/발급. Step2 결과 = 성공 `Callout` → 토큰 시크릿(masked + `Eye` 토글 + `Copy`) → 업로드 URL(`Copy`+QR `Popover`) → 경고 "이 토큰은 지금만 표시됩니다".
- **핵심 컴포넌트:** shadcn `Dialog`, `Form`(rhf+zod), `RadioGroup`, `Input`, `Select`, `Button`, `Separator`, `Badge`, `Popover`, `Sonner`. Tremor `Callout`. Lucide `Building2`,`UserCog`,`KeyRound`,`Copy`,`Eye`/`EyeOff`,`QrCode`,`Link2`,`TriangleAlert`,`Clock`,`Loader2`,`Ban`.
- **상태:** default(폼) · submitting(`Loader2`) · success(Step2 reveal) · error(폼 인라인 + `OkcError`) · **슬롯 10/10**(폼 진입 차단, `Ban`, 발급 disabled) · **freeze 후 승인 존재**(`Callout` "새 소스 등록 시 이전 taxonomy/cluster 승인이 무효화됩니다") · PROJECT_BUSY.
- **정직성:** 시크릿 1회 노출 후 해시 저장(NFR-SEC-1). 라벨 비검증. 발급=인증 아님(슬롯 부여).
- **매핑:** FR-UP-1,2,4 · 원 ④. **FOCAL:** 아니오(1회성 reveal 마이크로 인터랙션만).

### E2-3 — 업로드 포털(토큰 랜딩) `/u/[token]`
- **레이아웃:** Upload Shell. 상단 얇은 org 로고 바 + 중앙 단일 `Card`("개인 Vault 업로드"). 토큰 상태 Badge + 대상 정보(프로젝트명·슬롯 `#n`·owner 라벨 prefill) + 허용 포맷/크기 요약(`Info`) + Primary `Button`"업로드 시작"(→E2-4). 차단 시 상태별 Empty/Error 카드.
- **핵심 컴포넌트:** shadcn `Card`,`Badge`,`Button`,`Alert`,`Separator`,`Tooltip`,`Skeleton`,`Input`(토큰 수동입력 폴백). Tremor `Callout`. Lucide `ShieldCheck`,`ShieldAlert`,`ShieldX`,`Clock`,`Ban`,`Upload`,`FileArchive`,`Info`.
- **상태:** loading(검증 중) · valid(진행 CTA) · error(`OkcError` 분기: `TOKEN_INVALID`(`ShieldX`)/`TOKEN_EXPIRED`(`Clock`, "관리자에게 재발급 요청")/`TOKEN_REVOKED`(`Ban`)/`SOURCE_CAP_EXCEEDED`) · `PROJECT_BUSY`(정보성 "업로드는 대기 큐로 처리").
- **매핑:** FR-UP-2(만료/폐기/무효 거부) · 원 ④. **FOCAL:** 아니오.

### E2-4 — Vault 업로드 & 검증 `/u/[token]/upload`
- **레이아웃(2-컬럼):** 좌 = 대형 Dropzone → 파일 카드(이름/크기/포맷 아이콘) → owner_display_name `Input` → 업로드 `Progress`. 우 = **검증 리포트 패널**(체크리스트 항목별 상태 아이콘 + 요약 배지 + 실패 시 `Accordion` 상세).
- **검증 체크리스트(FR-UP-3 + okc-core hostile-input 중복 정직 반영):** ①포맷(zip/tar.zst만) ②크기 상한(`CategoryBar` 사용량/초과) ③Markdown 비율(.md 개수/비율; 비-md 첨부/Canvas/Base는 **경고** "병합 출력에 포함되지 않음 — Markdown 전용 materializer") ④경로 안전성(`../`·절대경로 거부) ⑤심볼릭 링크 거부(발견 목록 나열) ⑥중복 검출(content hash 대조).
- **핵심 컴포넌트:** shadcn `Card`, Dropzone(react-dropzone 커스텀), `Input`,`Progress`,`Badge`,`Alert`,`Accordion`,`Tooltip`,`Button`,`Sonner`. Tremor `ProgressBar`/`CategoryBar`/`List`. Lucide `Upload`,`FileArchive`,`FileText`,`FolderTree`,`CircleCheck`,`XCircle`,`TriangleAlert`,`ShieldAlert`,`Hash`,`Loader2`,`Ban`,`Gauge`.
- **인터랙션:** 드래그오버 하이라이트 · 파일 선택/교체/제거 · 업로드 시작(멀티파트) · 검증 결과 **순차 reveal**(각 체크가 위→아래로 통과, 절제된 애니메이션) · 실패 상세 `Accordion` · "다시 업로드".
- **상태:** empty(초기 dropzone) · dragging · uploading(`ProgressBar`+`Loader2`) · validating(순차 체크) · success(→E2-5 CTA) · error(`OkcError` 분기: `UPLOAD_FORMAT_UNSUPPORTED`/`UPLOAD_TOO_LARGE`(`Gauge`)/`PATH_UNSAFE`·`SYMLINK_REJECTED`(`ShieldAlert`+위반 목록)) · warn 비차단(`CONTENT_NOT_MARKDOWN`·`DUPLICATE_SOURCE`) · 세션 중 `TOKEN_EXPIRED`(차단) · `PROJECT_BUSY`(등록 직렬화 재시도 배너).
- **매핑:** FR-UP-3,2,4 · 원 ④. **FOCAL:** 아니오 — E2의 "mini-polish"(악성 입력 방어 순차 통과 시연).

### E2-5 — 업로드 완료 & 소스 등록 상태 `/u/[token]/done`
- **레이아웃(중앙 Card):** 성공 헤더(`PackageCheck`) + 등록 상세(정의 리스트: SourceId · owner_display_name · content hash 단축+`Copy` · 파일 수/.md 수 · 슬롯 `#n / 10` `CategoryBar` · 등록 시각) + 후속 안내("통합 대기 — admin이 소스 freeze 후 통합 실행") + freeze 후 등록 시 `Callout`("이전 taxonomy/cluster 승인이 무효화되었습니다").
- **핵심 컴포넌트:** shadcn 정의 리스트(Descriptions 패턴), `Badge`,`Separator`,`Button`,`Tooltip`,`Skeleton`,`Sonner`. Tremor `Callout`/`Metric`/`CategoryBar`. Lucide `PackageCheck`,`Hash`,`Fingerprint`,`Layers`,`Building2`,`UserCog`,`CircleCheck`,`TriangleAlert`,`Ban`.
- **상태:** loading(add_source Job) · success · `DUPLICATE_SOURCE`(등록 스킵/no-op) · `SOURCE_CAP_EXCEEDED`(11번째 등록 거부, `Ban`, `CategoryBar` 만석) · **STALE_APPROVALS**(freeze 재실행 무효화 경고) · `PROJECT_BUSY`(대기+폴링) · error(`OkcError`).
- **정직성:** 등록=소스 추가일 뿐, 컴파일(no-clobber 병합)은 admin 몫. owner_display_name=장식 라벨. 중복은 hash 기반 dedup.
- **매핑:** FR-UP-4(착지+add_source+≤10), 연계 FR-INT-1/2(freeze 무효화) · 원 ④. **FOCAL:** 아니오.

---

## Epic E3 — 통합 오케스트레이션 (FR-INT-1,2,7,8 + RBAC)

okc-core `IntegrationCheckpoint`(`NeedsProvider→NeedsSources→NeedsDisclosure→NeedsTaxonomy→NeedsClusters→ReadyToCompile→Verified`)의 오케스트레이션 쉘. **critique fix #4 반영:** IA 사이드바(7항목)와 1:1이 되도록 구 "소스+오케스트레이션 병합 허브"를 **Overview / Sources / Integration 3개 전용 화면으로 분리**한다.

정직 노출 제약: RBAC(변경성=admin만), 소스 ≤10, freeze-then-run 무효화, 직렬화 실행(`PROJECT_BUSY`=배너), 차단 findings 시 컴파일 게이트, 컴파일 결정론·no-clobber, `OkcError` code/category 분기, publish barrier 이전만 취소, `curator_id`(비검증 라벨).

**체크포인트 Tracker vs Job Phase 진행 — 시각 구분(consistency issue #9 해소):**
- **파이프라인 단계(macro, 상시):** Tremor `Tracker` **블록** 7칸 = IntegrationCheckpoint. Overview·Integration 헤더에 상주. 색: 완료=grass·현재=indigo·대기=slate·경고(stale)=amber·차단/에러=red. 라벨 "파이프라인 단계".
- **실행 진행(micro, 실행 중에만):** **세로 Steps 리스트**(각 단계 `CircleDashed`→`CircleDot`→`CircleCheck`, 현재 `Loader2`) + Tremor `ProgressBar`/`ProgressCircle`. 절대 블록 Tracker를 쓰지 않는다. 라벨 "실행 진행". phase = preflight→embedding→candidate→synthesis→critic.

### E3-1 — 프로젝트 목록 & 통합 대시보드 `/projects`
- **레이아웃:** 헤더(타이틀 + "새 프로젝트"`Button`+`Plus` + ⌘K) → 상단 Tremor `Grid` 스탯 로우 4~5(총/진행 중/리뷰 대기/Verified/차단 합계) → `DataTable`(프로젝트 행).
- **핵심 컴포넌트:** Tremor `Card`/`Metric`/`Text`/`Grid`/`CategoryBar`(소스 사용률), shadcn `Table/DataTable`,`Badge`,`Button`,`Command`,`DropdownMenu`(행 액션),`Skeleton`,`Alert`,`Tooltip`,`Sonner`. Lucide `FolderGit2`,`Plus`,`Search`,`CircleDot`,`CircleCheck`,`ShieldAlert`,`Loader2`,`Ban`.
- **인터랙션:** 행 클릭 → `/projects/[id]`(Overview); "새 프로젝트" → E3-2 `Dialog`; ⌘K 점프; 행 DropdownMenu(열기/이름변경/삭제 확인).
- **상태:** loading(skeleton) · empty(생성 CTA) · populated · error(`OkcError` `Alert`) · **PROJECT_BUSY**(해당 행 배지 + 전역 재시도 `Alert`) · RBAC(비-admin은 애초에 도달 불가 → 403).
- **표시 데이터:** 프로젝트명 · curator_id(mono) · 소스 `n/10`(CategoryBar) · 현재 체크포인트 Badge · 차단 findings 수 · 마지막 업데이트 · stale 여부. 스탯 5종.
- **매핑:** FR-INT-1,2,8 · FR-AUTH-3,4 · 원 ①. **FOCAL:** 아니오.

### E3-2 — 새 프로젝트 생성 모달 `/projects/new`
- **레이아웃:** `Dialog` — 프로젝트명 `Input` + `curator_id` `Input`(세션 admin 식별자 프리필) + 안내 `Callout` + 취소/생성.
- **핵심 컴포넌트:** shadcn `Dialog`,`Form`(rhf+zod),`Input`,`Label`,`Button`,`Alert`/Callout,`Tooltip`,`Sonner`. Lucide `Plus`,`Info`,`Loader2`,`TriangleAlert`.
- **상태:** default(프리필) · validating · submitting(`Loader2`) · success(→`/projects/[id]` + `Sonner`) · error(`OkcError` 예 `INVALID_ARGUMENT`). Callout: "curator_id는 okc-core가 검증하지 않는 표시용 라벨입니다(권한 강제는 okc-web RBAC 담당)."(FR-AUTH-4)
- **매핑:** FR-INT-1, FR-AUTH-4 · 원 ①. **FOCAL:** 아니오.

### E3-3 — 프로젝트 Overview `/projects/[id]` (신설, IA Overview 정본)
- **페르소나/목적:** 프로젝트의 랜딩·요약. 현재 상태, 다음 액션, 파이프라인 위치를 3초에 파악. (구 병합 허브의 "개요" 역할을 전용 화면으로 승격 — critique missing_screens 해소.)
- **레이아웃:** Breadcrumb + 프로젝트 헤더(이름·curator_id·상태 Badge·배너 슬롯). ①상단 **파이프라인 단계 Tracker**(블록 7칸, Taxonomy/Clusters 칸은 "리뷰로 이동 →" 딥링크). ②지표 행 Tremor `Card`+`Metric`(소스 `n/10`·차단 findings·승인 완료/대기·verify 상태). ③**Next-Action Card**(현재 체크포인트에 따라 단 하나의 권장 행동: "소스 고정"/"통합 실행"/"리뷰 필요 ⛔N"/"컴파일 준비 완료"/"서빙 공개 가능"과 해당 화면 CTA). ④최근 감사 로그 요약(`ScrollText`).
- **핵심 컴포넌트:** shadcn `Breadcrumb`,`Card`,`Badge`,`Button`,`Separator`,`Tooltip`,`Skeleton`,`ScrollArea`. Tremor `Tracker`,`Card`/`Metric`,`CategoryBar`. Lucide `LayoutDashboard`,`ArrowRight`,`ScanSearch`,`Hammer`,`Radio`,`Lock`,`ScrollText`,`CircleDot`.
- **인터랙션:** Next-Action CTA/Tracker 칸 클릭 → 해당 섹션 딥링크. 상태별 자동 라우팅 힌트.
- **상태:** loading · needs-sources(소스 추가 유도) · ready-to-run · running(pulse) · needs-review(⛔N 강조) · ready-to-compile · verified · **stale**(무효화 배너) · PROJECT_BUSY · error.
- **매핑:** FR-INT-2,8 · 원 ①. **FOCAL:** 아니오(단, 파이프라인 Tracker는 E3-5와 시각 규약 공유해 focal로 이어지는 진입선).

### E3-4 — 소스 & Freeze `/projects/[id]/sources`
- **페르소나/목적:** 소스 목록(≤10) 관리·고정. (구 병합 허브의 소스 파트를 전용 화면으로.)
- **레이아웃:** 헤더(제목 + `n/10` `CategoryBar` + "소스 추가"`Button`, 10이면 disabled+`Tooltip`). `DataTable`(소스 행). 우측/하단 액션: "소스 고정(Freeze)"(`Lock`) — 고정 시 소스 편집 잠금. 소스는 unfrozen에서만 삭제.
- **핵심 컴포넌트:** shadcn `Table/DataTable`,`Badge`,`Button`,`AlertDialog`(freeze/삭제 → E3-6),`HoverCard`(hash/메타),`DropdownMenu`,`Tooltip`,`Alert`,`Skeleton`,`Sonner`. Tremor `CategoryBar`. Lucide `Files`,`Upload`,`FileText`,`Hash`,`Trash2`,`Lock`(freeze)/`LockOpen`(unfrozen),`Building2`,`UserCog`,`Ban`,`History`(무효화),`Clock`.
- **컬럼:** owner_display_name(부서/개인 아이콘) · 포맷(zip/tar.zst/dir) · 크기 · content hash(`HoverCard`) · 추가일 · 고정 여부 · 중복/symlink 거부 표시 · 액션.
- **인터랙션:** "소스 추가"(E2 착지 파일 선택 → add_source; 10 도달 시 disabled + cap `Tooltip`; 11번째 차단 `Alert`) · 삭제(unfrozen만) · **"소스 고정"**(→E3-6 확인 → 잠금 → E3-5 실행 활성) · 소스 hash HoverCard.
- **상태:** loading · empty-sources(업로드/추가 유도) · sources<10 · **sources=10**(추가 disabled + cap 안내) · unfrozen(편집 가능) · **frozen(읽기전용)** · **stale**(소스 변경 → 무효화 amber 배지 "재실행 필요") · PROJECT_BUSY(변경 잠금) · error(`OkcError`).
- **매핑:** FR-INT-1(≤10), FR-INT-2(freeze), FR-UP-4 · 원 ①. **FOCAL:** 아니오.

### E3-5 — 통합 오케스트레이션 & 진행 모니터 `/projects/[id]/integration` ★FOCAL★
- **페르소나/목적:** 실행 트리거 + 파이프라인 단계 표시 + **수 분 소요 장기 통합의 라이브 관측**(Job 이벤트 폴링). 데모에서 "진짜 엔진이 돈다"는 확신을 주는 화면. **Provider/Disclosure 게이트도 여기서 명시**(critique missing_screens 해소).
- **레이아웃:**
  - 상단: 프로젝트 헤더 + **파이프라인 단계 Tracker**(블록 7칸, 현재 pulse) + 배너 슬롯.
  - **Provider & Disclosure 카드**(NeedsProvider/NeedsDisclosure 결정 표면):
    - NeedsProvider → 배지 "AI provider: 서버 env-var로 사전 프로비저닝됨(A-2) — 승인 불필요"(`Server`, neutral/grass). 자동 충족.
    - NeedsDisclosure → `allow_remote_provider`=true면 **원격 provider 고지 확인 `Switch`**(`remote_disclosure_confirmed`, `Info`/`ShieldAlert`); 로컬/offline이면 배지 "로컬 provider — 고지 불필요". → Tracker의 두 단계가 데모에서 "유령 스텝"이 되지 않도록 설명.
  - 실행 컨트롤: "통합 실행"(`Play`) — freeze 후에만 활성. 실행 시 하단 **진행 모니터** 자동 오픈(`?job=live`).
  - **진행 모니터(FOCAL 처리)**: 상단 **실행 진행 세로 Steps 리스트**(preflight→embedding→candidate→synthesis→critic; 현재 `Loader2` pulse) + Tremor `ProgressBar`/`ProgressCircle`(embedding n/총 등). 중앙 Tremor `Card` 3(경과 시간/처리 소스 수/현재 phase). 하단 `ScrollArea` **append-only 이벤트 로그**(타임스탬프 + phase + `code` Badge, 최신이 아래로, 모노스페이스). 우상단 "취소"(publish barrier 이전만; 이후 disabled + `Tooltip`).
- **핵심 컴포넌트:** shadcn `Card`,`Badge`,`Button`,`Switch`,`Separator`,`ScrollArea`,`Collapsible`(이벤트 상세),`Tooltip`,`Alert`,`AlertDialog`(취소),`Sonner`. Tremor `Tracker`(파이프라인 단계, 블록),`ProgressBar`/`ProgressCircle`,`Card`/`Metric`/`Text`. Lucide `Workflow`,`Play`,`Activity`,`Loader2`,`CircleDot`,`CircleCheck`,`CircleDashed`,`TriangleAlert`,`Ban`(취소 불가),`TerminalSquare`(로그),`Server`,`Info`,`ArrowRight`,`Clock`.
- **인터랙션:** freeze 확인(E3-6) → "통합 실행" → Job 시작 → 모니터 자동 오픈 · 단계 완료마다 `Sonner` 토스트 · Taxonomy/Clusters 도달 시 스텝에 "리뷰로 이동 →"(E4 딥링크) · 완료 시 ReadyToCompile → "Compiled Vault로 이동"(E5-1 딥링크) · 취소(publish barrier 이전만).
- **상태:** idle(실행 전) · **running**(Steps 애니메이션 + 로그 append) · phase-progress(ProgressBar) · **paused-by-busy**(`PROJECT_BUSY` 큐 대기 배지 + 자동 재시도 안내) · succeeded(ReadyToCompile/Verified) · **failed**(`OkcError` code/category + retryable에 따라 재시도/문의) · **cancel-blocked**(publish barrier 이후) · **stale**(상위 변경 무효화 배너).
- **표시 데이터:** 현재 phase, 진행률, 경과/예상 시간, 처리 소스 수, 이벤트 로그(각 이벤트 stable code/category Badge — 문자열 파싱 아님), provider/disclosure 상태.
- **매핑:** FR-INT-2,7(딥링크 인계),8 · NFR-AVAIL-1(취소 경계), NFR-OBS-1 · 원 ①.
- **FOCAL: 예(E3 focal).** wow: 실행 진행 Steps의 단계 pulse + 부드러운 phase 전환 + 라이브 append 로그(모노 + code Badge 컬러) + `Sonner` 토스트 스트림. "장기 작업이 안전하게 관측·취소된다"를 즉시 체감. **채팅/쿼리 UI가 아니라 엔진 Job의 관측 창**임을 명확히.

### E3-6 — 소스 고정/변경 무효화 경고 (AlertDialog on `/sources`·`/integration`)
- **목적:** freeze-then-run·hash-bound 승인을 정직하게 강제. (a) 소스 고정 확인, (b) 고정/승인 후 소스·설정 변경 시 **모든 하위 승인 stale**를 destructive 경고.
- **레이아웃:** `AlertDialog` — 헤더(`TriangleAlert`+제목) → destructive `Callout` → **영향받는 승인 목록 `Table`**(무효화될 taxonomy/cluster 승인) → 동의 `Checkbox` → 취소/진행.
- **핵심 컴포넌트:** shadcn `AlertDialog`,`Alert`/Callout(destructive),`Badge`,`Button`,`Checkbox`,`Table`,`Separator`,`Sonner`. Lucide `TriangleAlert`,`Lock`(freeze),`RefreshCw`(재실행),`History`(무효화될 승인),`ShieldAlert`.
- **상태:** freeze-confirm · **invalidation-warning**(stale 예고) · destructive-confirm(체크 후 진행) · no-op(실변경 없음) · **blocked-while-busy**(`PROJECT_BUSY` 중 변경 차단).
- **매핑:** FR-INT-1,2 · C-4 · 원 ①. **FOCAL:** 아니오(정직성 포인트).

---

## Epic E4 — Conflict/Critic 리뷰 & 해소 (FR-INT-3,4,5,6 + 7 게이트, 8)

데모 하이라이트. **모든 라우트 admin-only**(contributor는 nav 미렌더, 직타 403). okc-core 제약: 승자 선택 없음(승인/waive/omission/regenerate 4 결정 표면), Major/Critical **waive 불가**(regenerate만 해소), 차단 findings 존재 시 컴파일 거부, 모순 보존(양측 근거), freeze-then-run 무효화, 직렬화(PROJECT_BUSY), `OkcError` 분기.

**심각도 아이콘 정본(critique fix #3, `design-system.md §5.3` 단일 사전):** Critical=`OctagonAlert`(red solid) · Major=`TriangleAlert`(red soft) · Minor=`CircleAlert`(amber soft) · Waived=`Undo2` · Contradiction=`GitCompareArrows`/`Scale`(violet) · Approved=`CircleCheck` · Blocked=`Ban`. (구 초안의 `AlertOctagon`/Minor=`Info`/Major=`CircleAlert` 오매핑 전면 수정.)

### E4-1 — 리뷰 개요 & 컴파일 게이트 `/projects/[id]/review`
- **목적:** 리뷰 스코어보드 + 컴파일 가능 여부 판정. "왜 컴파일이 막혔는가"를 3초에 설명. **컴파일 실행 트리거는 여기 없음** — 게이트 상태만 보이고 준비되면 `/compiled`로 **딥링크**(critique fix #9: 트리거 단일화).
- **레이아웃:** PageHeader(체크포인트 Badge)+Breadcrumb. ①**컴파일 게이트 배너(히어로)** 풀폭 `Alert` 3분기: 적색 "컴파일 거부 — 차단 findings N건 (Critical C · Major M)" / 황색 "승인 대기 N 클러스터" / 녹색 "컴파일 준비 완료". ②지표 행 Tremor `Card`+`Metric`(소스 `n/10`·클러스터 수·승인 완료/대기·차단 findings·보존 모순·waive 건수). ③Tremor `Tracker`(클러스터별 상태) + `CategoryBar`(Critical/Major/Minor 분포). ④클러스터 상태 `DataTable`(클러스터명·상태 Badge·severity 칩·"리뷰"→E4-3). ⑤**stale 배너**(무효화 목록+링크). ⑥하단 CTA: 준비 완료 시 "Compiled Vault로 이동 →"(E5-1 딥링크), 차단 시 disabled + `Tooltip` "미해소 Major/Critical 때문에 컴파일이 거부됩니다."
- **핵심 컴포넌트:** shadcn `Card`,`Badge`,`Button`,`Alert`,`Table/DataTable`,`Tooltip`,`Separator`,`Breadcrumb`,`ScrollArea`,`Skeleton`. Tremor `Card`/`Metric`/`Text`/`Tracker`/`CategoryBar`/`BarList`/`Callout`. Lucide `ScanSearch`,`Ban`(거부),`ShieldCheck`(준비),`OctagonAlert`(Critical),`TriangleAlert`(Major),`CircleAlert`(Minor),`CircleDashed`(대기),`GitCompareArrows`(모순),`Layers`,`RefreshCw`,`ArrowRight`.
- **인터랙션:** 클러스터 행 → E4-3. Taxonomy 미승인이면 클러스터 섹션 잠금 + "Taxonomy 먼저 승인"(→E4-2). stale 링크 → 재검토.
- **상태:** loading · empty(통합 미실행 → E3-5 유도) · NeedsTaxonomy(클러스터 잠금) · NeedsClusters(황색) · ReadyToCompile(녹색+딥링크 활성) · **blocked**(적색, 딥링크 disabled+tooltip) · **stale**(황색 배너) · PROJECT_BUSY(재시도 배너+`Sonner`) · error(`OkcError` `Callout`).
- **매핑:** FR-INT-3,4,5,6(개요), FR-INT-7(게이트), 8 · 원 ③. **FOCAL:** 아니오(base로 강등, critique fix #6). 게이트 배너 적↔녹 전환만 담백한 강조.

### E4-2 — Taxonomy Proposal 편집·승인 `/projects/[id]/review/taxonomy`
- **목적:** taxonomy proposal 검토·편집 → `approve_taxonomy(edited_clusters, rationale)` 승인 → 클러스터별 synthesis 생성 Job.
- **레이아웃(2단):** 좌 = 제안 트리(`Collapsible` 클러스터→노트, 각 노트에 소스 배지 `owner_display_name`로 부서/개인 혼합 provenance 노출). 우 = 편집 상세(이름 편집·노트 이동/이관). 툴바: rename/merge/split/create/delete, 노트 move(`Command` 검색), 드래그 핸들. 편집 diff Badge("편집됨"/"병합"/"이동"). rationale `Textarea`(승인 필수). 승인 바: "Taxonomy 승인 & Synthesis 생성" + freeze-then-run 경고 `Alert`.
- **핵심 컴포넌트:** shadcn `Collapsible`,`Card`,`Textarea`,`Input`,`Button`,`Badge`,`DropdownMenu`,`Dialog`(merge/split),`Command`,`Tooltip`,`Alert`,`ScrollArea`,`Separator`,`Skeleton`. Tremor `Callout`. Lucide `FolderTree`,`Layers`,`Split`,`GitMerge`(merge),`Pencil`,`Move`,`Plus`,`Trash2`,`GripVertical`,`Info`,`ArrowRight`,`Check`.
- **인터랙션:** 편집 누적(dirty) · rationale 비면 승인 disabled(zod `min(1)`) · 승인 → job → 인라인 진행(실행 진행 Steps + `Sonner`) → E4-3 유도.
- **상태:** loading · proposal ready · **edited(dirty)** · validation(rationale 미입력 차단) · approving(진행) · approved(read-only + "클러스터 리뷰로 이동") · **stale**(황색 배너) · PROJECT_BUSY · error.
- **매핑:** FR-INT-3, FR-INT-2(stale), 8 · 원 ③(전단). **FOCAL:** 아니오. wow 포인트: 소스 배지로 "여러 부서 Vault가 하나의 taxonomy로 섞이는" 통합 가치 시각화.

### E4-3 — 클러스터 리뷰 워크스페이스 `/projects/[id]/review/clusters/[clusterId]` ★FOCAL(메인 히어로)★
- **목적:** 클러스터별 **synthesis + critic findings** 검토 + 4개 결정 액션(승인/Minor waive/omission 제안/regenerate). Major/Critical=차단·waive 불가를 명백히. 데모 클라이맥스.
- **레이아웃(정본 — critique fix #5 해소, 3-패널 워크벤치):** `ResizablePanelGroup`
  - **좌 패널(클러스터 리스트):** `ScrollArea`. 항목 = 클러스터명 + 상태 Badge(승인됨/대기/차단/재생성중) + severity 롤업(`OctagonAlert`/`TriangleAlert`/`CircleAlert` 도트) + findings 카운트. 상단 필터 `DropdownMenu`("차단만"/"미승인만") + `Command` 검색.
  - **중앙 패널(클러스터 상세):** `Tabs` = **[Synthesis · 모순(Contradictions) · Omission 제안 · Taxonomy]**. 헤더에 상태 + severity `CategoryBar` + 차단 시 "차단됨" 리본.
    - **Synthesis 탭:** 렌더된 Markdown 합성 결과(react-markdown). **provenance 마커는 노트/파일 단위 출처로 한정**(스팬 단위 아님) — critique fix #7. 하단 고지 `Callout`: "provenance는 파일 단위 보증입니다(스팬 단위 아님, NFR-DET-1)."
    - **모순 탭(focal 처리, 구 E4-S3 흡수):** DiffViewer식 좌우 split panel — 좌=소스 A(owner_display_name 배지 + 주장 + 증거 발췌 + provenance 링크), 우=소스 B(동일), 중앙 거터 `Scale`/`GitCompareArrows`(violet 프레이밍). 상단 라벨 "보존된 모순 — 승자 없음(no winner)". **Approve/Reject/승자 버튼 절대 없음.** 하단: "양측 모두 `knowledge/`에 보존되며 `.okc/` provenance에 기록됩니다." 유일한 우회는 클러스터 Regenerate(→E4-4). 여러 모순은 `?contra=<id>` 딥링크 + prev/next.
    - **Omission 제안 탭:** synthesis 제안 누락 후보, 사유 입력 후 수용.
    - **Taxonomy 탭:** 이 클러스터의 taxonomy 컨텍스트(읽기).
  - **우 패널(Findings 인스펙터, 상시 고정 레일):** critic findings 목록. finding별 `Card` = severity Badge + 카테고리 + 설명 + 증거 발췌(`HoverCard` 원문 미리보기) + 관련 노트 참조. **Major/Critical 카드 = `Lock` 아이콘 없이 red 배지 + "waive 불가" 라벨 + `Callout`("regenerate로만 해소"); Waive 버튼은 렌더하되 disabled + `Tooltip` "Major/Critical은 waive 불가 — regenerate로만 해소".** **Minor 카드만 활성 "Waive" 버튼.** (좁은 뷰포트에선 우 레일이 `Sheet`로 축소.)
  - **하단 sticky 결정 액션바:** `Button` 4종 — **승인**(`approve_cluster`) · **Minor Waive**(사유 `Dialog`) · **Omission 제안**(사유 `Dialog`) · **Regenerate**(→E4-4). **승인 버튼은 미해소 Major/Critical 존재 시 disabled + `Tooltip`.** 상시 미니 배너 "차단 findings 존재 → 컴파일 거부".
- **결정 다이얼로그:** 승인 `Dialog`(`approve_cluster(omission_rationales, minor_waivers)` 집계 확인) · Minor Waive `Dialog`(대상 finding + `Textarea` 사유 필수, zod) · Omission `Dialog`(대상 노트/주장 + 사유 필수) · Regenerate(→E4-4, `AlertDialog` 경고 "재생성은 현재 synthesis 폐기 + 하위 승인 무효화").
- **핵심 컴포넌트:** shadcn `ResizablePanelGroup`/`ResizablePanel`,`Card`,`Tabs`,`Badge`,`Button`,`Dialog`,`Sheet`,`AlertDialog`,`Textarea`,`Label`,`Tooltip`,`HoverCard`,`Alert`,`Collapsible`,`ScrollArea`,`Separator`,`DropdownMenu`,`Command`,`Skeleton`,`Sonner`. Tremor `CategoryBar`,`Callout`,`Text`,`BadgeDelta`. Lucide `OctagonAlert`(Critical),`TriangleAlert`(Major),`CircleAlert`(Minor),`Undo2`(waive),`ScanSearch`,`CircleCheck`(승인),`RefreshCw`/`Sparkles`(regenerate),`EyeOff`(omission),`MessageSquare`(feedback),`GitCompareArrows`/`Scale`(모순),`FileText`,`ChevronRight`,`Filter`. (Markdown: react-markdown — shadcn 외 명시.)
- **인터랙션:** 좌측 선택 → 우측/중앙 갱신(URL path param 동기화). Minor Waive → 사유 → "waived" 칩+`HoverCard`(사유/작성자), 차단 카운트 제외. Omission → 사유 → 제안 목록 반영. 승인 → 차단 0일 때만 `approve_cluster` → "승인됨"(grass·잠금). Regenerate → E4-4.
- **상태:** loading · pending · Minor-only(승인 활성) · **blocked(Major/Critical)**(승인 disabled, regenerate 강조, waive-불가 메시징, "컴파일 거부" 배너) · approved(read-only grass, 재오픈만) · **regenerating**(스피너, 액션 비활성, 토스트) · waived/omission submitted(칩·사유 반영) · **stale**(황색 "재검토 필요") · validation(사유/피드백 미입력 차단) · PROJECT_BUSY(재시도) · error(예 `APPROVAL_REQUIRED` → `Callout`).
- **표시 데이터:** 클러스터 목록·상태, synthesis 본문, critic findings(severity/카테고리/증거/노트참조), 보존 모순 양측 근거, omission 후보, waive 사유 이력.
- **매핑:** FR-INT-4, FR-INT-5(결정+un-waivable+거부), FR-INT-6(모순 탭), 8 · 원 ③.
- **FOCAL: 예(핵심 wow).** wow: severity 배지 대비 + disabled 승인 버튼 + "컴파일 거부" 배너 + 모순 탭의 좌우 대칭 split("승자 없음")이 "왜 막혔는지"와 제품 철학을 한 화면에 각인. Minor waive vs Major/Critical regenerate 강제의 대비가 심사 포인트.

### E4-4 — Regenerate 피드백 & 재생성 결과 diff `/projects/[id]/review/clusters/[clusterId]/regenerate`
- **목적:** Major/Critical 해소의 **유일 경로**. 피드백 → `regenerate_cluster(feedback)` Job → 재생성 synthesis + re-critic findings를 이전과 diff.
- **레이아웃(3-step):** Step1 피드백 `Textarea`(필수) + 가이드 칩 + 현재 차단 findings 체크리스트 + "재생성 실행". Step2 진행(실행 진행 Steps + `ProgressBar`, publish barrier 이전 취소 `AlertDialog`). Step3 결과 비교 좌우 split — Before(이전 synthesis+findings) vs After(새 synthesis+새 findings). 해소=취소선/녹색, 신규 회귀=강조. severity delta(`BadgeDelta`). 차단 0이면 "승인 가능"→E4-3 복귀.
- **핵심 컴포넌트:** shadcn `Textarea`,`Button`,`Card`,`Badge`,`Alert`,`AlertDialog`,`Progress`,`Tabs`,`ResizablePanel`,`ScrollArea`,`Tooltip`,`Skeleton`,`Sonner`. Tremor `ProgressBar`,`Callout`,`BadgeDelta`,`Metric`,`Text`. Lucide `RefreshCw`,`MessageSquare`,`Sparkles`,`GitCompareArrows`,`CircleCheck`,`XCircle`,`OctagonAlert`,`Loader2`,`Ban`(취소),`ArrowRight`.
- **상태:** input(피드백 필수) · submitting/queued · running(취소 가능) · cancelled(barrier 이전) · **complete-resolved**(녹색 "차단 0 → 승인 가능") · **complete-remaining/regressed**(추가 regenerate 유도) · stale · PROJECT_BUSY · error.
- **매핑:** FR-INT-5(주),4(re-critic),8 · NFR-AVAIL-1(취소 경계) · 원 ③. **FOCAL:** 아니오(base로 강등, critique fix #6). severity delta "Critical 2→0"가 게이트를 적→녹으로 바꾸는 순간은 담백하게.

---

## Epic E5 — 병합 Vault 서빙 & okc-mcp 계약 (FR-SRV-1..3, NFR-DET-1)

`CompiledVaultManifest` 전제. **모든 라우트 admin-only**(contributor 접근 불가 → 403; critique rbac_issues 해소 — "contributor read-only 조회"·"contributor Switch 비활성" 문구 전면 삭제). 정직 반영: compile 결정론·offline·Markdown 전용·no-clobber(`knowledge/`+`legacy/`+`.okc/`), verify=**내부 일관성 증명**(발행자 진위 보증 아님), 모순 보존(승자 없음), provenance **파일 단위**, freeze-then-run 무효화, okc-mcp **미구현**(계약만·채팅/쿼리 UI 없음), 재임베딩 필요.

### E5-1 — Compiled Vault (컴파일 실행 + 병합 Vault 브라우저) `/projects/[id]/compiled`
- **목적:** **컴파일 트리거 단일 지점(critique fix #9)** + 산출물 read-only 브라우저(구 E3-S6 + E5-S1 통합). 3영역 트리는 **여기 한 곳에만** 렌더.
- **레이아웃(상태 2모드):**
  - **Pre-compile(ReadyToCompile & 차단=0):** 게이트 상태 `Card`(승인 완료·차단 0 확인) + **"컴파일 실행"(`Hammer`)** → `AlertDialog` 확인("결정론적·offline·Markdown 전용·기존 산출 no-clobber" 고지) → 실행 중 `ProgressBar`(취소 불가). 차단>0이면 버튼 disabled + `Ban` + "리뷰로 이동"(E4-1).
  - **Post-compile(3-pane Resizable 브라우저):** 좌=파일 트리(`knowledge/`·`legacy/`·`.okc/` 3 루트, `Collapsible` Tree). 중=노트 뷰어(Breadcrumb + `Tabs`[렌더링 Markdown / Raw / Frontmatter]). 우=노트 메타 레일(verify 상태 Badge·기여 소스 요약 `owner_display_name×N`·contradiction 유무·**"Provenance & Verify 상세"** → E5-2 `?note=` 딥링크). 상단 매니페스트 요약 Tremor `Metric`(총 노트 수·verify PASS 비율·소스 `n/10`·매니페스트 hash).
- **핵심 컴포넌트:** shadcn `Resizable`,`ScrollArea`,`Collapsible`(Tree),`Breadcrumb`,`Command`(경로/노트 검색),`Tabs`,`Card`,`Badge`,`AlertDialog`,`Button`,`HoverCard`,`Tooltip`,`Separator`,`Skeleton`,`Alert`,`Sonner`. Tremor `ProgressBar`,`Card`/`Metric`,`List`. Lucide `Package`(Compiled Vault nav),`Hammer`(compile),`PackageCheck`(산출 결과),`FolderTree`,`Folder`/`FolderOpen`,`FileText`,`CornerUpRight`(legacy 스텁),`ScrollText`(.okc),`Hash`,`ShieldCheck`(verify),`Info`,`FileWarning`(no-clobber 충돌),`ArrowRight`.
- **인터랙션:** 컴파일 실행(위) · 트리 노드 클릭 → 본문 로드 · `legacy/` 스텁 → "이 노트는 `knowledge/<대상>`으로 리다이렉트됨" + 이동 · `.okc/` → read-only 감사 JSON · ⌘K 경로 검색 · 우 레일 "Provenance & Verify 상세" → E5-2. **read-only 강제: 편집·삭제·이름변경 액션 없음.**
- **상태:** loading · empty(산출물 없음 → "리뷰(E4)에서 승인 후 컴파일" CTA) · **blocked**(차단 findings → `APPROVAL_REQUIRED` 거부, `Ban`+사유) · not-approved · confirm(no-clobber/결정성 고지) · compiling(취소 불가) · **success**(매니페스트 3디렉터리 트리·hash·provenance 수·verify 요약) · **no-clobber-collision**(`FileWarning` 거부/신규 경로 안내) · **stale**(무효화 배너 → 재컴파일 필요) · note에 contradiction 존재 시 정보 배지 · PROJECT_BUSY · error(`OkcError`).
- **표시 데이터:** 매니페스트 hash·파일/노트 수·3영역 트리·provenance 레코드 수·per-note verify 배지·기여 소스·contradiction 플래그·출력 절대경로·컴파일 타임스탬프.
- **매핑:** FR-INT-7(knowledge/legacy/.okc·no-clobber·결정성), FR-SRV-1(목록·본문 조회), FR-SRV-2(inline verify 진입), FR-INT-6(모순 플래그), FR-INT-2(stale), 8 · NFR-DET-1, NFR-PORT-1 · 원 ①(→⑤ 인계). **FOCAL:** 아니오(정갈한 정보밀도 베이스). E5-2로의 매끄러운 진입이 데모 흐름을 만든다.

### E5-2 — Provenance & Verify 뷰 `/projects/[id]/serving/verify` ★FOCAL(두 번째 히어로)★
- **목적:** 선택 노트의 **계보 추적선** + `verify()` 무결성 + `explain()` 파생 + 보존 모순을 한 화면에서 보는 감사 뷰. (E5-1 우 레일에서 `?note=`로 진입.)
- **레이아웃:** 상단 노트 헤더 + verify 상태 Badge(`BadgeCheck`/`ShieldAlert`) + 결정성 해시(mono+`Copy`) + **authenticity 고지 `Callout`(상시)**: "verify는 산출물의 내부 일관성 증명이며 발행자 진위 보증이 아닙니다(NFR-DET-1)." + **provenance 세분성 고지: 파일 단위 보증(스팬 아님).** 중앙 히어로 = **Provenance Lineage 그래프**(좌 소스 노트 노드[owner_display_name·SourceId·content hash] → 중앙 클러스터/합성 노드 → 우 컴파일 노트 노드, 곡선 엣지, draw-in 애니메이션, 노드 `HoverCard` 상세). 하단 `Tabs`: Provenance(기여 소스 `Table`) / Verify(Tremor `Tracker` per-check PASS/FAIL 스트립) / Explain(`Accordion` 파생 단계) / Contradictions(split panel, 승자 없음).
- **핵심 컴포넌트:** Provenance Lineage 그래프(커스텀 SVG / React Flow 스타일 node-edge, draw-in) · Contradiction split panel(DiffViewer식 dual-column, "모순은 보존됩니다(승자 선택 불가)" 라벨) · Tremor `Card`/`Metric`,`Tracker`(per-check verify),`Callout` · shadcn `Tabs`,`Card`,`Badge`,`Accordion`,`HoverCard`,`Table`,`Alert`,`Tooltip`,`Button`(Copy hash/소스 이동/서빙 계약 보기),`Skeleton`,`Sonner`. Lucide `Waypoints`,`Route`,`GitMerge`,`Fingerprint`,`ShieldCheck`,`ShieldAlert`,`BadgeCheck`,`CircleCheck`,`XCircle`,`History`,`Scale`,`GitCompareArrows`,`Copy`.
- **인터랙션:** 그래프 노드 클릭 → 소스 노트(E5-1) 이동 또는 `HoverCard` · content hash `Copy` · `explain()` `Accordion`(임베딩→후보→taxonomy→합성→compile 근거) · "서빙 계약 보기"→E5-4.
- **상태:** loading(그래프+verify Skeleton) · verify **PASS**(통과 배지+해시 강조) · verify **FAIL**(실패 체크 강조 + **"재컴파일로만 해소"** — critique fix #8: 'waive' 용어 제거) · **contradiction 존재**(split 양측 병렬, 승자 UI 없음) · 단일 소스 노트(추적선 1개 명시) · stale(무효화 경고) · PROJECT_BUSY · error(`OkcError`).
- **표시 데이터:** 계보 그래프(소스→클러스터→합성→노트), 기여 소스(owner_display_name·SourceId·hash), verify 체크 결과, explain 파생 단계, 모순 양측 근거, 결정성 해시.
- **매핑:** FR-SRV-2(verify/explain/provenance), FR-INT-6(모순 보존), FR-INT-7(결정성), NFR-DET-1 · 원 ⑤.
- **FOCAL: 예(두 번째 wow).** wow: lineage 그래프 draw-in + 단일 accent로 계보 강조 + verify PASS 명료 배지 + 모순 split "승자 없음" 정직함. 나머지 UI는 미니멀로 눌러 이 화면만 돌출.

### E5-3 — 서빙 개요 & publish `/projects/[id]/serving`
- **목적:** 병합 Vault를 read-only HTTP API/URL로 공개(FR-SRV-1). 서빙 섹션의 랜딩(서브탭: 개요 / Provenance·Verify[E5-2] / okc-mcp 계약[E5-4] — route-backed).
- **레이아웃:** 상단 서빙 상태 `Card`(상태 Badge LIVE/OFFLINE/STALE + **공개 `Switch`(admin-only)** + Vault 무결성 요약 Tremor `Tracker` 파일 수·verify PASS 비율). 중단 `Tabs`: 엔드포인트(base URL + `GET /api/serving/tree`·`/note?path=`·`/verify`·`/explain?path=` `Table` + curl 예시 + 로컬 디렉터리 절대경로) / 예시 응답(list/read/verify JSON 코드 블록 + `Copy`). 하단 out-of-scope 고지 `Callout`.
- **핵심 컴포넌트:** shadcn `Card`,`Switch`,`Tabs`,`Table`,`Badge`,`Dialog`(공개/중단 확인),`Alert`,`Button`(Copy·재컴파일),`Separator`,`Skeleton`,`Sonner`, code block+Copy. Tremor `Card`/`Metric`,`Tracker`,`ProgressBar`,`Callout`. Lucide `Radio`,`Server`,`Globe`,`Link2`,`Copy`,`ExternalLink`,`RefreshCw`,`Terminal`,`Code2`.
- **인터랙션:** 공개 `Switch` 토글 → `Dialog` 확인 → publish(LIVE)/중단(OFFLINE), **admin만**. 각 경로/curl/디렉터리 `Copy`. "재컴파일"(stale 시 → E5-1). verify 미통과 시 공개 차단.
- **상태:** empty(산출물 없음 → "먼저 컴파일하세요" → E5-1) · offline(기본) · publishing(로딩) · live(read-only 배지) · **stale**(재컴파일 CTA, freeze-then-run) · `PROJECT_BUSY` · no-clobber 안내 · error(`OkcError`). (contributor 뷰잉 상태 없음 — 403.)
- **매핑:** FR-SRV-1, FR-AUTH-3(admin-only publish), FR-INT-2(stale),8 · NFR-DET-1 · 원 ⑤. **FOCAL:** 아니오.

### E5-4 — okc-mcp 연동 계약 `/projects/[id]/serving/contract`
- **목적:** okc-mcp가 소비할 **위치·형식 계약** 제시(FR-SRV-3). 청킹/임베딩/vector index/쿼리는 out-of-scope 명시. **채팅/쿼리 UI 없음.**
- **레이아웃:** okc-mcp 소비 가이드 `Card`(소비 위치[로컬 디렉터리 경로 또는 `/api/serving/*` 엔드포인트]·형식[Markdown-only 트리 + `.okc/` 감사]) + `Tabs`(cURL / 경로) + 계약 스키마 `Card`. 상단 **`Badge: 이연(Deferred)`** 명확 표기. 하단 `Callout`: "okc-mcp는 미구현입니다. 청킹·임베딩·vector index·쿼리는 out-of-scope이며, 코어 임베딩은 일시적이라 컴파일 산출물을 **재임베딩**해야 합니다."
- **핵심 컴포넌트:** shadcn `Card`,`Tabs`,`Table`,`Badge`,`Button`(Copy·계약 문서 열기),`Alert`,`Separator`,`Skeleton`, code block+Copy. Tremor `Callout`. Lucide `Plug`/`PlugZap`,`Code2`,`FileJson`,`Link2`,`Copy`,`ExternalLink`,`Info`.
- **상태:** default(계약 표시) · empty(산출물 없음 → 컴파일 유도) · error(`OkcError`). **okc-mcp UNIMPLEMENTED 상시 고지.**
- **매핑:** FR-SRV-3(계약·out-of-scope) · 원 ⑤. **FOCAL:** 아니오(고유 가치 프레이밍: "완성물이 아니라 정직한 계약").

### read-only 서빙 API `/api/serving/*` (UI 없음)
- `GET /api/serving/tree` · `GET /api/serving/note?path=` · `GET /api/serving/verify` · `GET /api/serving/explain?path=` — 전부 read-only, okc-mcp(비인간) 소비 대상. FR-SRV-1,2.

---

## §9 데모 동선 워크스루 (End-to-End, 요구사항 §9 5단계 매핑)

| §9 단계 | 담당 화면 | 핵심 시연 포인트 |
|---|---|---|
| **1. 업로드** (FR-UP-1..4) | admin: E2-1→E2-2(토큰 발급, 슬롯 `#n/10`) → contributor: E2-3(셸 없는 토큰 랜딩) → **E2-4**(zip 드롭 + 검증 순차 통과, 심볼릭/경로 위반 정직 노출) → E2-5(SourceId·hash·슬롯 등록) | 앱 셸 없는 contributor 동선 = RBAC 물리 분리. ≤10 상한 상시 노출. |
| **2. 통합 시작** (FR-AUTH-3, FR-INT-1,2) | E1-1(admin 로그인) → E3-1 → E3-2(프로젝트 생성, curator_id 고지) → E3-3(Overview next-action) → **E3-4**(소스 2~3개, `n/10`) → E3-6(freeze 확인) → **E3-5 ★**(Provider/Disclosure 설명 + "통합 실행" → 라이브 진행 모니터) | freeze-then-run. Provider/Disclosure 스텝이 "유령"이 아님을 배지로 설명. FOCAL #1(라이브 스텝퍼). |
| **3. 리뷰·선택** (FR-INT-3..6) | E3-5에서 Taxonomy/Clusters 도달 → E4-1(게이트 스코어보드) → E4-2(taxonomy 승인) → **E4-3 ★**(클러스터 리뷰: Minor waive vs Major/Critical 차단→컴파일 거부; 모순 탭 "승자 없음") → 필요 시 **E4-4**(regenerate → Critical 2→0) | FOCAL #2(결정 표면). 승자 선택 없음·un-waivable·모순 보존을 화면으로 증명. |
| **4. 컴파일** (FR-INT-7) | E4-1 게이트 녹색 → **E5-1**("컴파일 실행" 단일 트리거 → 결정론·no-clobber 확인 → 3영역 트리 산출) | 컴파일 트리거·트리 렌더가 한 곳(`/compiled`). |
| **5. 서빙** (FR-SRV-1..3) | E5-1 우 레일 → **E5-2 ★**(lineage 그래프 + verify PASS + 모순 split) → E5-3(엔드포인트 + publish `Switch`) → E5-4(okc-mcp 계약 `이연` 배지) | FOCAL #3(provenance/verify). 서빙=계약만, 채팅/쿼리 UI 없음. |

사이드바 배지(`n/10`·`⛔N`·`✓`)와 Run-State Pill이 전 과정 상태를 요약 — 심사위원이 화면 구조만으로 파이프라인·권한·제약을 읽는다.

---

## FOCAL "wow" 화면 (정확히 3개) — 적용할 폴리시

1. **E3-5 통합 진행 모니터** — 실행 진행 세로 Steps의 단계 pulse(현재 `Loader2`, 완료 `CircleCheck`) + Tremor `ProgressBar` 부드러운 채움 + 라이브 append 이벤트 로그(모노스페이스, code Badge 컬러) + `Sonner` 토스트 스트림 + phase 전환 220~250ms ease-out. 파이프라인 블록 Tracker와 실행 Steps를 **시각적으로 구분**(블록 vs 리스트)해 두 상태 혼동 방지.
2. **E4-3 클러스터 리뷰 워크스페이스** — 3-패널 워크벤치. severity 배지 대비(`OctagonAlert` red solid / `TriangleAlert` red soft / `CircleAlert` amber) + 미해소 시 승인 버튼 disabled + 상단 "컴파일 거부" 배너 attention pulse(1회) + 모순 탭 좌우 대칭 split(violet 프레이밍, `Scale` 중앙, "승자 없음" 라벨). waive 불가 카드의 disabled Waive + Tooltip으로 제약을 "보이게".
3. **E5-2 Provenance & Verify** — lineage 그래프 draw-in(엣지 경로 애니메이션, transform/opacity만) + Verified 배지 scale-in(160ms) + 결정성 해시 강조 + 모순 split "승자 없음". authenticity 고지 `Callout` 상시. 컨페티/글로우 금지.

(E4-1 게이트 배너·E4-4 severity delta·E2-4 검증 순차 통과는 focal이 아닌 **담백한 강조**로만 처리.)

---

## Out-of-Scope UI (명시적 미설계 — 요구사항 §6 정합)

- **okc-mcp 리트리벌/쿼리/채팅 UI**: 없음. E5-4는 위치·형식 **계약**만 제시(`이연(Deferred)` 배지). 청킹·임베딩·vector index·쿼리·MCP 툴 표면은 out-of-scope, 재임베딩 필요 명시. (FR-SRV-3, Q7=A)
- **Federation / 10 소스 초과 UI**: 없음. 11번째는 발급·등록·추가 단계에서 차단만 표시(설계 방향은 문서화, 구현 이연). (C-3, Q3=X)
- **멀티테넌시 / 조직 전환 / at-rest 암호화 설정 UI**: 없음. 단일 조직·단일 워크스페이스 가정, 로컬/신뢰 환경. (Q1=A, NFR-STORE-1)
- **OIDC/SSO 로그인 UI**: 없음. 토큰/비밀번호만. (Q2=B)
- **비-Markdown 자료 뷰어(첨부/Canvas/Base) · 완전 link-rewrite · OKCPack UI**: 없음. E2-4에서 비-md는 경고로 표기(병합 출력 미포함). (Q10=A)
- **증분 재통합 UI**: 없음. freeze-then-run 단일 실행만.
- **contributor용 앱 셸/대시보드/`/me/uploads`**: 없음(설계 제외). contributor 유일 접점은 `/u/[token]` 3화면.