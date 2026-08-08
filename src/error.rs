use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("CoInitializeEx failed: HRESULT 0x{0:08x}")]
    CoInit(i32),

    #[error("{context}: HRESULT 0x{hr:08x}")]
    Hr { context: &'static str, hr: i32 },

    #[error("windows: {0}")]
    Windows(#[from] windows_core::Error),

    #[error("field {0:?} missing")]
    MissingField(String),

    #[error("field {name:?}: expected {expected}, got {got}")]
    TypeMismatch {
        name: String,
        expected: &'static str,
        got: &'static str,
    },
}
