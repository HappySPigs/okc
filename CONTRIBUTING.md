# 개발과 검토

제품 범위는 **OKC에 적합한 Obsidian 원본 Vault 작성**이다. 먼저
[요구사항](aidlc-docs/inception/requirements.md)과 [현재 상태](aidlc-docs/state.md)를 읽는다.
OKC compiler·승인 정책을 이 저장소에서 복제하지 않는다.

```sh
npm ci
npm run check
npm pack
```

변경을 작게 나누고 관련 요구사항·사용자 동작·테스트 근거를 같은 PR에 남긴다.
새 CLI/도구/설정은 README와 추적표를 갱신한다. 깨지는 변경은 CHANGELOG에 표시한다.
설계 대안이 제품 경계를 바꾸면 AI-DLC의 Inception 요구사항에 먼저 기록한다.

테스트는 합성 데이터와 임시 Vault만 사용한다. 개인 Vault, 실제 출처의 비공개 전문,
설정·백업·토큰을 commit하거나 이슈에 첨부하지 않는다. 다른 프로젝트의 코드는 복사하기 전에
라이선스와 귀속 조건을 별도로 검토한다.

쓰기 변경은 stale hash, create no-replace, 외부 backup, path/type boundary와
실제 stdio 호출을 확인한다. 안전성 한계를 숨기거나 테스트 없이 지원 OS를 추가하지 않는다.
문서 변경은 링크와 실행 예시를 확인한다. 공개 배포는 로컬 pack 성공과 다른 단계다.
