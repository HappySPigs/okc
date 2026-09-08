//! DisclosureProvider — RISK-01 informed-consent 고지 문안(정적, DEC-U5-14).
//!
//! `disclosure_text()` 는 **4대 필수 내용**(연속성 · 비가역성/forward-only · 클라 필터 없음 ·
//! 로컬 평문 산출물[config 토큰 포함])을 모두 포함한다. 최종 법적/UX 문안은 Code Generation
//! 이월이나, 4개 요소의 부재는 회귀로 차단한다(PROP-U5-09).
//!
//! 순수 정적 표면으로서 panic-free-total 을 컴파일타임으로 강제한다.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

/// RISK-01 고지 문안(4대 필수 내용 포함). 최초 실행 시 확인(acknowledge) 대상이다.
pub const DISCLOSURE_TEXT: &str = "\
Watcher 상시 동의 고지 (RISK-01)

1. 연속성: 상시 동의와 자동 동기화가 활성화되면, 이후 볼트의 모든 변경(나중에 추가되는 \
비밀·파일 포함)이 추가 확인 없이 자동으로 서버에 업로드됩니다.

2. 비가역성 / forward-only 철회: 업로드된 콘텐츠는 회수할 수 없습니다. 동의 철회는 이후 \
업로드만 차단하는 forward-only 이며, 서버측 삭제·강제는 보장되지 않습니다.

3. 클라이언트 측 필터 없음: raw 볼트 전체(비밀·개인정보 포함)가 클라이언트 측 민감 콘텐츠 \
필터나 사전검사 없이 그대로 전송됩니다.

4. 로컬 평문 산출물: 히스토리·매니페스트·SyncState·로그, 그리고 config 파일의 평문 토큰이 \
OS 보안 저장소 밖 로컬 디스크에 평문으로 존재합니다.";

/// RISK-01 고지 문안을 반환한다(R-CG-DISCLOSURE).
pub fn disclosure_text() -> &'static str {
    DISCLOSURE_TEXT
}
