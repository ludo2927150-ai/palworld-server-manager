use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("E/S : {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON : {0}")]
    Json(#[from] serde_json::Error),
    #[error("HTTP : {0}")]
    Http(#[from] reqwest::Error),
    #[error("ZIP : {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("INI invalide : {0}")]
    Ini(String),
    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;

// Les commandes Tauri exigent une erreur sérialisable.
impl serde::Serialize for Error {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}
