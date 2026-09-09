# OKC 해커톤 이미지 사용 가이드

게시 사이트가 SVG를 지원해도 기본 업로드는 PNG를 권장합니다. SVG는 해상도와 무관한 벡터 편집 원본으로 함께 보관했습니다.

## 권장 게시 순서

| 순서 | 파일 | 유형 | 권장 위치 | 캡션 |
|---:|---|---|---|---|
| 1 | `assets/00-okc-hero.png` | 생성형 개념 이미지 | 제목 바로 아래 | 각자의 Vault는 그대로. 승인된 지식만 하나의 검증 가능한 revision으로. |
| 2 | `assets/09-why-okc-comic-v2.png` | 생성형 원화 + 벡터 레터링 | 도입부 | 개인 기록 → 부서 아카이브 → 코딩 에이전트 재사용의 순환. |
| 3 | `assets/01-trust-pipeline.png` | 벡터 개념도 | 핵심 아이디어 | AI는 제안하고, 코드는 검증하며, 사람만 승인합니다. |
| 4 | `assets/10-core-merge-logic.png` | 벡터 수식 도식 | core 핵심 로직 | 점수는 후보를 찾고, 완전성은 발행 여부를 결정합니다. |
| 5 | `assets/02-okc-ecosystem.png` | 벡터 에코시스템 설계 | 전체 구조 | 작성부터 버전 고정 소비까지 네 모듈이 하나의 루프로 움직입니다. |
| 6 | `assets/11-mcp-hooks-friendly.png` | 벡터 UX 도식 | 사용자 경험 | 기록은 대화에서, 동기화는 백그라운드에서, 재사용은 다음 작업에서. |
| 7 | `assets/04-sources-freeze.png` | 실제 UI | 데모 1 | 세 팀의 입력을 먼저 고정해 승인 기준점을 만듭니다. |
| 8 | `assets/05-compiled-vault.png` | 실제 UI | 데모 2 | 승인된 계획이 읽기 전용 Vault와 proof 파일로 컴파일됩니다. |
| 9 | `assets/06-provenance-verify.png` | 실제 UI | 데모 3 | 선택한 compiled note, 프로젝트 source owner 라벨, Verified 상태. |
| 10 | `assets/03-overview.png` | 실제 UI | 데모 결과 | 3 sources · frozen · blocking 0 · verify PASS. |
| 11 | `assets/08-engineering-proof.png` | 벡터 수치 카드 | 엔지니어링 증거 | 모듈별 최신 로컬 검증 결과와 four-module bridge. |

`assets/07-serving.png`는 발행 API와 LIVE 상태를 보여주는 보조 화면입니다. 게시글이 길어질 경우 생략해도 핵심 서사는 유지됩니다.

## 실제 화면 캡처의 범위

화면 캡처는 세 개의 부서 샘플 노트를 실제 `okc-web`과 native `okc-core` 경로로 ingest, freeze, approve, compile, publish, verify해 만들었습니다. 콘텐츠 생성에는 deterministic synthetic provider를 사용했습니다.

- 증명하는 것: web/core workflow, native binding, review/compile/publish/verify 계약, provenance UI
- 증명하지 않는 것: live third-party LLM의 문장 품질, 실제 고객 데이터, 운영 배포 성능

이 설명은 게시물의 “실제 동작 화면” 첫 문단에 이미 포함되어 있습니다.

## Hero 생성 정보

- 모드: built-in `image_gen`
- 분류: `ads-marketing`
- 최종 파일: `assets/00-okc-hero.png`

최종 프롬프트:

```text
Use case: ads-marketing
Asset type: 16:9 hero visual for a hackathon showcase post about OKC, Obsidian Knowledge Compilation
Primary request: Create a sophisticated editorial technology illustration showing many independent personal knowledge vaults being transformed through a rigorous verification pipeline into one trustworthy compiled knowledge base.
Scene/backdrop: deep charcoal-black canvas with subtle paper grain; the scene flows from scattered note stacks and connected knowledge fragments on the left, through a precise transparent compilation-and-verification chamber in the center, into a calm, ordered luminous knowledge graph and bound archive on the right.
Subject: the central chamber visibly separates an upper AI proposal stream from a lower deterministic verification gate; thin provenance threads remain visibly connected from every output on the right back to source fragments on the left; a restrained human approval gesture is represented by one clear checkpoint seal.
Style/medium: premium isometric editorial illustration, precise technical geometry, cinematic but credible, polished enough for a top-tier developer hackathon, no generic stock-photo look.
Composition/framing: wide horizontal composition with clear left-to-right transformation; strong focal point in the center; generous clean negative space around the edges for cropping; readable even as a thumbnail.
Lighting/mood: controlled indigo and cyan light against obsidian black, trustworthy, rigorous, quietly impressive.
Color palette: obsidian black, slate, electric indigo, cool cyan, small warm amber accent only at the human approval checkpoint.
Materials/textures: glass, brushed dark metal, fine paper, subtle luminous data paths.
Constraints: no written words, no letters, no numbers, no logos, no brand marks, no UI screenshots, no watermarks; visually communicate deterministic compilation, human-in-the-loop approval, provenance, and verification without text.
Avoid: humanoid robots, floating brains, cheesy AI imagery, neon overload, cyberpunk cityscapes, padlock clichés, messy unreadable details.
```

## 6컷 만화 생성 정보

- 모드: built-in `image_gen` 원화 + SVG 벡터 레터링
- 분류: `illustration-story`
- 최종 파일: `assets/09-why-okc-comic-v2.png`
- 편집 원본: `assets/09-why-okc-comic-v2.svg`
- 텍스트 없는 생성 원화: `assets/09-why-okc-comic-v2-art.png`

원화 생성에 사용한 최종 프롬프트:

```text
Use case: illustration-story
Asset type: six-panel landscape comic for a Korean hackathon showcase explaining the full value loop of Obsidian MCP and OKC
Primary request: Create one cohesive, very cute six-panel comic page in a strict 3-column by 2-row grid. The story must focus on capturing coding knowledge in a personal Obsidian vault, turning many personal vaults into a trustworthy department archive, and letting a coding agent reuse that archived knowledge in future work. Contradiction handling is only a small secondary detail.
Scene/backdrop: cozy miniature software-team world with local note vaults visualized as small rounded book houses, a shared department archive as a welcoming glowing library, and knowledge moving as colorful ribbons and note cards.
Characters: use the same consistent cast across panels — a cheerful chibi developer in a blue hoodie and round glasses, a product teammate in a warm apricot cardigan, an operations teammate in a mint-green sweater, a curator in a navy jacket with a tiny amber approval pin, and a small friendly non-humanoid coding-agent mascot shaped like a glowing indigo desktop sprite with simple eyes. Large heads, short limbs, expressive faces, rounded props.
Panel 1: while the developer codes with the small coding-agent mascot, useful decisions, terminal commands, debugging discoveries, and a solved incident become tidy note cards that the mascot places directly into the developer's personal Obsidian vault book-house. Show the benefit of effortless knowledge capture during real work.
Panel 2: in a later coding session, the developer faces a similar problem. The mascot instantly retrieves the right prior note from the personal vault, and the developer happily resumes without re-explaining or starting research from zero. Include one large empty speech bubble for the mascot.
Panel 3: several teammates each have their own colorful vault book-house. A cute automated courier/conveyor representing continuous sync carries sealed revision bundles from every vault into one shared department archive. Everyone keeps their own workspace while knowledge gathers centrally.
Panel 4: inside the department archive, the curator reviews note bundles while a friendly transparent verification machine checks source ribbons, hashes, and approvals before placing them on organized shelves. Preserve colored provenance threads back to the original vaults. Show one tiny conflicting pair marked with a restrained amber warning flag, but keep archiving and trust as the main focus.
Panel 5: on a new project, the coding-agent mascot connects to the department archive through a glowing purple MCP cable and pulls out a past architecture decision, a deployment runbook, and a troubleshooting checklist. The developer uses them beside a new code project. Include one large empty speech bubble for the mascot.
Panel 6: the new feature is completed smoothly, the team celebrates, and a fresh learning note flows back into the personal vault and then toward the department archive. Show a clear circular flywheel connecting personal vault, shared archive, coding agent, new work, and new knowledge.
Style/medium: charming Korean webtoon meets cozy picture-book illustration, soft pastel cel shading, thick clean rounded outlines, sticker-like details, gentle paper texture, expressive chibi characters, delightful but polished enough for a developer hackathon. Clearly non-photorealistic and much cuter than a realistic office illustration.
Composition/framing: exact 3 by 2 grid, generous cream-colored gutters, consistent panel sizes. Leave a calm dark-navy caption strip across the top 16 percent of every panel for later Korean vector lettering. Keep panel actions large, simple, and readable at thumbnail size. Preserve generous open space inside the speech bubbles in panels 2 and 5.
Lighting/mood: warm, optimistic, collaborative, with a brief thoughtful verification moment in panel 4. The final panel should feel joyful and cyclical.
Color palette: cream paper, pastel indigo, sky blue, mint, apricot, soft amber, dark navy outlines, small green verification accents.
Constraints: absolutely no written text, no letters, no words, no numbers, no logos, no brand marks, no watermarks; blank speech bubbles only; exactly six panels; consistent character identities; the coding agent must be a cute abstract desktop sprite, not a humanoid robot.
Avoid: photorealism, realistic anime proportions, cyberpunk, dark dystopia, superhero action, humanoid robots, floating brains, corporate stock-photo look, dense tiny UI, extra panels, panel overlap, illegible symbols.
```

정확한 한국어 레터링은 생성 후 SVG에서 추가했습니다.

1. Obsidian MCP로 결정·명령·해결법을 바로 기록
2. 다음 세션에는 지난 지식을 곧바로 재사용
3. 각자의 Vault는 유지하고, 지식은 부서 아카이브로
4. OKC가 출처·승인·모순을 지켜 함께 정리
5. 코딩 에이전트가 과거 결정과 런북을 재사용
6. 새 배움도 돌아와 조직 지식이 자랍니다

말풍선은 “지난 해결법을 찾았어요!”, “지난 설계와 런북을 찾았어요!”입니다. 마지막 태그라인은 “개인의 기록이 부서의 자산이 되고, 다음 개발의 출발점이 됩니다.”입니다.

## 업로드 팁

- 대표 이미지는 `00-okc-hero.png`를 사용합니다.
- 도입부에는 `09-why-okc-comic-v2.png`, 시스템 설명에는 `02-okc-ecosystem.png`를 사용합니다.
- 기술 심사자가 많은 플랫폼에서는 `10-core-merge-logic.png`를 trust pipeline 바로 다음에 배치합니다.
- 사용자 가치 설명에는 `11-mcp-hooks-friendly.png`를 사용합니다.
- 실제 작동 증거에서 가장 중요한 이미지는 `06-provenance-verify.png`입니다.
- UI 캡처는 글자가 읽히도록 본문 전체 폭으로 배치합니다.
- 이미지 자동 압축이 강한 플랫폼에서는 SVG 원본을 별도로 첨부하거나 1600px 이상 폭을 유지합니다.
- 실제 UI와 개념 이미지를 캡션에서 명확히 구분합니다.

벡터 편집 원본은 각각 `10-core-merge-logic.svg`, `11-mcp-hooks-friendly.svg`입니다.
