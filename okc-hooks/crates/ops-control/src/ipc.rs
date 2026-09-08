//! IPC 경계 seam — 전송 주소(`IpcEndpoint`) + 전송 오류(`IpcError`) + 3-트레이트 seam
//! (`IpcListener`/`IpcConnector`/`IpcStream`) + mac/Linux Unix 도메인 소켓 구현.
//!
//! 프레이밍은 **8바이트 big-endian 길이 프리픽스 + CBOR 페이로드** 1프레임이다(D-U7B-05, U6
//! `history.rs` `LEN_PREFIX=8` 관례 미러). 절단 프레임/선언 길이 초과/디코드 실패는 **패닉하지
//! 않고** `IpcError::Protocol` 로 반환한다(R-U7B-05, 회복력). 프레이밍 헬퍼(`read_frame_from`/
//! `write_frame_to`)는 `Read`/`Write` 위 순수 로직이라 소켓 없이 결정적으로 테스트된다.
//!
//! 실 소켓 구현은 `std::os::unix::net`(mac/Linux 1차)이며 owner-only 디렉터리(0700) 하위에
//! best-effort `chmod 0600` 으로 바인딩한다(R-U7B-07). Windows 명명 파이프는 동일 seam 뒤에서
//! 이연(스텁)되어 non-unix 에서도 크레이트가 컴파일된다.
//!
//! 소켓 I/O 를 수행하는 모듈이므로 순수 lint-gate 는 두지 않는다(단, 어떤 경로에서도
//! `unwrap`/`expect`/`panic` 을 쓰지 않는다).

use std::io::{Read, Write};
use std::path::PathBuf;

/// 단일 프레임 페이로드 상한(바이트). 선언 길이가 이를 초과하면 `Protocol` 로 거부한다(회복력).
pub const MAX_FRAME_BYTES: u64 = 8 * 1024 * 1024;

/// 길이 프리픽스 폭(바이트).
const LEN_PREFIX: usize = 8;

/// IPC 엔드포인트 주소 — 조립 루트(U8)가 data-dir 관례로 해소해 주입한다(D-U7B-06).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IpcEndpoint {
    /// Unix 도메인 소켓 경로(mac/Linux, 1차 구현).
    SocketPath(PathBuf),
    /// Windows 명명 파이프(이연/스텁).
    NamedPipe(String),
}

/// 전송 오류 taxonomy(U7b 소유, D-U7B-12). 도메인 오류(`ControlError`)와 분리된다(R-U7B-09).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum IpcError {
    /// 서버 바인딩 실패(경로 점유/권한 등).
    #[error("소켓 바인딩 실패")]
    Bind,
    /// 소유자-전용 권한 설정/검증 실패.
    #[error("소유자-전용 권한 설정 실패")]
    Permission,
    /// 클라이언트 연결 실패(데몬 미기동/소켓 부재 -> CLI exit 2).
    #[error("데몬에 연결할 수 없음")]
    Connect,
    /// 프레이밍 위반: 절단/과대 길이/디코드 실패(패닉 없음, R-U7B-05).
    #[error("프로토콜 위반(프레이밍/디코드)")]
    Protocol,
    /// 버전 불일치(전송 계층 관측 시).
    #[error("프로토콜 버전 불일치")]
    VersionMismatch,
    /// 기타 하위 I/O 실패.
    #[error("IPC I/O 실패: {0}")]
    Io(String),
}

/// 양방향 프레임 채널(read/write 프레임). 1 연결 = 1 요청/응답(MVP 동기).
pub trait IpcStream {
    /// 길이-프리픽스 1프레임을 수신한다(절단/과대 -> `Protocol`).
    fn read_frame(&mut self) -> Result<Vec<u8>, IpcError>;
    /// 페이로드 1프레임을 길이-프리픽스로 송신한다.
    fn write_frame(&mut self, payload: &[u8]) -> Result<(), IpcError>;
}

/// 서버 측(데몬) 리스너 — 다음 연결을 수락한다(블로킹).
pub trait IpcListener {
    /// 다음 연결을 수락한다.
    fn accept(&self) -> Result<Box<dyn IpcStream>, IpcError>;
}

/// 클라이언트 측(CLI) 커넥터 — 엔드포인트로 연결한다.
pub trait IpcConnector {
    /// 엔드포인트로 연결한다(데몬 미기동 -> `Connect`).
    fn connect(&self, endpoint: &IpcEndpoint) -> Result<Box<dyn IpcStream>, IpcError>;
}

/// 페이로드를 길이-프리픽스 프레임으로 기록한다(`Write` 위 순수 로직).
pub fn write_frame_to<W: Write>(writer: &mut W, payload: &[u8]) -> Result<(), IpcError> {
    let len = payload.len() as u64;
    if len > MAX_FRAME_BYTES {
        return Err(IpcError::Protocol);
    }
    writer
        .write_all(&len.to_be_bytes())
        .map_err(|e| IpcError::Io(e.to_string()))?;
    writer
        .write_all(payload)
        .map_err(|e| IpcError::Io(e.to_string()))?;
    Ok(())
}

/// 길이-프리픽스 프레임 1개를 읽어 페이로드를 복원한다(`Read` 위 순수 로직).
///
/// 프리픽스 부족/페이로드 절단(EOF)·선언 길이 초과는 패닉 없이 `Protocol` 로 반환한다(R-U7B-05).
pub fn read_frame_from<R: Read>(reader: &mut R) -> Result<Vec<u8>, IpcError> {
    let mut len_buf = [0u8; LEN_PREFIX];
    read_exact_protocol(reader, &mut len_buf)?;
    let len = u64::from_be_bytes(len_buf);
    if len > MAX_FRAME_BYTES {
        return Err(IpcError::Protocol);
    }
    let mut payload = vec![0u8; len as usize];
    read_exact_protocol(reader, &mut payload)?;
    Ok(payload)
}

/// `read_exact` 를 수행하되 EOF(절단)를 `Protocol` 로, 그 외 I/O 오류를 `Io` 로 매핑한다.
fn read_exact_protocol<R: Read>(reader: &mut R, buf: &mut [u8]) -> Result<(), IpcError> {
    match reader.read_exact(buf) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => Err(IpcError::Protocol),
        Err(e) => Err(IpcError::Io(e.to_string())),
    }
}

/// 현재 OS 에 맞는 프로덕션 커넥터(CLI 클라이언트용)를 반환한다.
#[cfg(unix)]
pub fn native_connector() -> Box<dyn IpcConnector> {
    Box::new(uds::UdsConnector)
}

/// 현재 OS 에 맞는 프로덕션 커넥터(CLI 클라이언트용)를 반환한다(non-unix: 스텁).
#[cfg(not(unix))]
pub fn native_connector() -> Box<dyn IpcConnector> {
    Box::new(stub::UnsupportedConnector)
}

/// 소유자-전용으로 엔드포인트에 바인딩한 프로덕션 리스너(데몬 서버용)를 반환한다.
#[cfg(unix)]
pub fn bind_native(endpoint: &IpcEndpoint) -> Result<Box<dyn IpcListener>, IpcError> {
    Ok(Box::new(uds::UdsListener::bind_owner_only(endpoint)?))
}

/// 소유자-전용으로 엔드포인트에 바인딩한 프로덕션 리스너를 반환한다(non-unix: 이연 스텁).
#[cfg(not(unix))]
pub fn bind_native(_endpoint: &IpcEndpoint) -> Result<Box<dyn IpcListener>, IpcError> {
    Err(IpcError::Bind)
}

/// mac/Linux Unix 도메인 소켓 구현(`std::os::unix::net`).
#[cfg(unix)]
mod uds {
    use std::fs;
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::net::{UnixListener, UnixStream};
    use std::path::Path;

    use super::{IpcConnector, IpcEndpoint, IpcError, IpcListener, IpcStream, read_frame_from, write_frame_to};

    /// UDS 기반 프레임 스트림.
    pub struct UdsStream {
        inner: UnixStream,
    }

    impl IpcStream for UdsStream {
        fn read_frame(&mut self) -> Result<Vec<u8>, IpcError> {
            read_frame_from(&mut self.inner)
        }

        fn write_frame(&mut self, payload: &[u8]) -> Result<(), IpcError> {
            write_frame_to(&mut self.inner, payload)?;
            self.inner.flush().map_err(|e| IpcError::Io(e.to_string()))
        }
    }

    /// UDS 리스너 — 소유자-전용 디렉터리(0700) 하위에 바인딩하고 소켓에 chmod 0600 을 적용한다.
    pub struct UdsListener {
        inner: UnixListener,
    }

    impl UdsListener {
        /// stale unlink -> 0700 부모 dir -> bind -> chmod 0600 순으로 owner-only 바인딩한다(R-U7B-07).
        pub fn bind_owner_only(endpoint: &IpcEndpoint) -> Result<Self, IpcError> {
            let path = socket_path(endpoint)?;
            // idempotent 기동: 잔존 소켓 경로를 unlink 후 재바인딩.
            let _ = fs::remove_file(path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(|_| IpcError::Bind)?;
                fs::set_permissions(parent, fs::Permissions::from_mode(0o700))
                    .map_err(|_| IpcError::Permission)?;
            }
            let listener = UnixListener::bind(path).map_err(|_| IpcError::Bind)?;
            fs::set_permissions(path, fs::Permissions::from_mode(0o600))
                .map_err(|_| IpcError::Permission)?;
            Ok(UdsListener { inner: listener })
        }
    }

    impl IpcListener for UdsListener {
        fn accept(&self) -> Result<Box<dyn IpcStream>, IpcError> {
            let (stream, _addr) = self.inner.accept().map_err(|e| IpcError::Io(e.to_string()))?;
            Ok(Box::new(UdsStream { inner: stream }))
        }
    }

    /// UDS 커넥터(CLI 클라이언트).
    pub struct UdsConnector;

    impl IpcConnector for UdsConnector {
        fn connect(&self, endpoint: &IpcEndpoint) -> Result<Box<dyn IpcStream>, IpcError> {
            let path = socket_path(endpoint)?;
            let stream = UnixStream::connect(path).map_err(|_| IpcError::Connect)?;
            Ok(Box::new(UdsStream { inner: stream }))
        }
    }

    /// 엔드포인트에서 소켓 경로를 추출한다(Windows 명명 파이프는 UDS 경로로 취급 불가 -> `Bind`).
    fn socket_path(endpoint: &IpcEndpoint) -> Result<&Path, IpcError> {
        match endpoint {
            IpcEndpoint::SocketPath(path) => Ok(path.as_path()),
            IpcEndpoint::NamedPipe(_) => Err(IpcError::Bind),
        }
    }
}

/// non-unix 이연 스텁(Windows 명명 파이프 미구현).
#[cfg(not(unix))]
mod stub {
    use super::{IpcConnector, IpcEndpoint, IpcError, IpcStream};

    /// 미지원 커넥터 — 항상 `Connect` 오류를 반환한다(un-defer = interprocess 크레이트).
    pub struct UnsupportedConnector;

    impl IpcConnector for UnsupportedConnector {
        fn connect(&self, _endpoint: &IpcEndpoint) -> Result<Box<dyn IpcStream>, IpcError> {
            Err(IpcError::Connect)
        }
    }
}
