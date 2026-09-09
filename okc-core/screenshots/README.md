# 데모 스크린샷 (캡처 대기)

> **상태: 이미지 미첨부.** 이 폴더는 실제 실행 스크린샷을 담기 위한 자리입니다.
> 아래 순서대로 OKC를 실행하고 각 단계 화면을 캡처해 지정된 파일명으로 이 폴더에
> 저장하세요. 이 저장소에는 아직 실제 캡처 이미지가 포함되어 있지 않습니다 —
> 자리표시(placeholder)나 합성 이미지를 넣지 마세요.

## 준비

```bash
rustup toolchain install 1.97.1
cargo +1.97.1 build --locked -p okc
```

`demo/`의 `VAULT_A` / `VAULT_B` / `VAULT_C`를 그대로 데모 입력으로 쓸 수 있습니다
(구성은 [`demo/README.md`](../demo/README.md) 참고). 로컬 provider(예: Ollama)를
연결하면 remote 호출 없이 전 과정을 재현할 수 있습니다.

## 캡처할 화면과 파일명

각 단계에서 아래 파일명으로 저장하세요. 순서는 [`guide/integration.md`](../guide/integration.md)의
Phase 1 → Phase 2 → offline 흐름과 같습니다.

| 파일명 | 무엇을 캡처하나 | 만드는 명령/화면 |
|---|---|---|
| `01-tui-vault-select.png` | cwd에서 `okc` 실행 시 뜨는 Vault 선택 TUI | `cd demo && ../target/debug/okc` |
| `02-taxonomy-review.png` | 제안된 taxonomy 검토 화면 | `review taxonomy show` (Phase 1) |
| `03-cluster-conflict.png` | 충돌/모순이 보존된 cluster 검토 화면 | `review cluster show CLUSTER_ID` (Phase 2) |
| `04-compile-success.png` | provider 없이 compile이 성공한 출력 | `okc ... compile --output ./CompiledVault` |
| `05-verify.png` | `verify`의 통과 출력 | `okc verify ./CompiledVault` |
| `06-explain.png` | 한 노트의 provenance 설명 | `okc explain ./CompiledVault knowledge/topic.md` |

## 캡처 시 주의

- 실제 화면 그대로 저장하세요. 값을 지어내거나 편집하지 마세요.
- 민감정보(실 API key, 개인 경로 등)가 화면에 보이면 캡처 전에 가리세요.
- 재현 가능한 결과를 원하면 `demo/`의 세 Vault + 로컬 provider 조합을 사용하세요
  (remote 호출·비용 없이 동작).
- 캡처를 추가한 뒤에는 이 문서 상단의 "상태" 문구를 실제 상태에 맞게 갱신하세요.
