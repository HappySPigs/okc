//! ConsentStore — `ConsentRecord` 원자 지속(temp+rename) + fail-safe 로드.
//!
//! 성공 전이만 U0 `encode`(CBOR) -> temp write + `fsync` -> `rename`(원자 커밋)으로 지속한다.
//! rename 이전 실패는 활성 상태 파일을 오염하지 않는다(부분쓰기 무손상). 기동 로드 시 `decode`
//! 실패/일관성 위반(손상)은 안전 기본으로 강등한다(R-CG-PERSIST, fail-safe: 차단 우선).
//!
//! 실제 파일 IO 를 수행하므로 순수 lint-gate 를 적용하지 않는다(U0 `store.rs` 관례).

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use foundation::{decode, encode};

use super::types::{ConsentRecord, sanitize};

/// `ConsentRecord` 를 원자적으로 지속·로드하는 스토어(지속 경로 보관).
///
/// 지속 경로는 U0 `ConfigProvider` 가 해소하는 데이터 디렉토리 하위이며, U8 조립루트가 확정해
/// 주입한다(U0 는 데이터 디렉토리 접근자를 노출하지 않으므로 경로는 하향 주입).
#[derive(Debug, Clone)]
pub struct ConsentStore {
    path: PathBuf,
}

impl ConsentStore {
    /// 지속 파일 경로로 스토어를 만든다.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        ConsentStore { path: path.into() }
    }

    /// 저장된 레코드를 로드한다(fail-safe). 파일 부재/디코드 실패/일관성 위반은 안전 기본으로 강등.
    ///
    /// 패닉 없이 항상 유효한 `ConsentRecord` 를 반환한다(의심 시 업로드 금지).
    pub fn load(&self) -> ConsentRecord {
        let bytes = match fs::read(&self.path) {
            Ok(bytes) => bytes,
            // 파일 부재/읽기 실패 -> 최초 상태(차단 우선).
            Err(_) => return ConsentRecord::initial(),
        };
        match decode::<ConsentRecord>(&bytes) {
            // 손상/일관성 위반은 sanitize 가 차단 우선으로 강등.
            Ok(record) => sanitize(record),
            Err(_) => ConsentRecord::initial(),
        }
    }

    /// 레코드를 temp+rename 로 원자적으로 지속한다(성공 전이에서만 호출).
    ///
    /// 실패(인코딩/쓰기/rename)는 `Err(())` 로 표면화하며 활성 상태 파일을 오염하지 않는다.
    /// 호출측 `ConsentGate` 가 이를 `ConsentError::PersistFailed` + 인메모리 롤백으로 처리한다.
    ///
    /// 반환은 의도적으로 `Result<(), ()>` 다 — 실패 원인(인코딩/IO)은 이 계층에서 구분하지 않고
    /// 호출측이 단일 `PersistFailed` 로 문맥을 부여하므로 별도 오류 타입을 두지 않는다(MVP).
    #[allow(clippy::result_unit_err)]
    pub fn persist(&self, record: &ConsentRecord) -> Result<(), ()> {
        let bytes = encode(record).map_err(|_| ())?;
        let temp_path = temp_path(&self.path);

        // temp 파일에 쓰고 fsync 후 원자적 rename 으로 커밋.
        let write_result = (|| -> std::io::Result<()> {
            let mut file = fs::File::create(&temp_path)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
            fs::rename(&temp_path, &self.path)
        })();

        match write_result {
            Ok(()) => Ok(()),
            Err(_) => {
                // rename 이전 실패 -> temp 정리(활성 파일 불변). 정리 실패는 무시(best-effort).
                let _ = fs::remove_file(&temp_path);
                Err(())
            }
        }
    }
}

/// 지속 경로에 대응하는 temp 경로(`<path>.tmp`)를 만든다.
fn temp_path(path: &Path) -> PathBuf {
    let mut os = path.as_os_str().to_os_string();
    os.push(".tmp");
    PathBuf::from(os)
}
