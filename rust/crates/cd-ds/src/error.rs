use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error("json parse: {0}")]
    Json(#[from] serde_json::Error),

    #[error("missing piece: {0}")]
    MissingPiece(&'static str),

    #[error("malformed {piece}: {detail}")]
    Malformed { piece: &'static str, detail: String },

    #[error("ref `{0}` is not defined in the DS")]
    UnknownRef(String),
}

pub type Result<T> = std::result::Result<T, Error>;
