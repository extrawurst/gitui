use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("`{0}`")]
    Generic(String),

    #[error("io error")]
    Io(#[from] std::io::Error),

    #[error("git error")]
    Git(#[from] git2::Error),

    #[error("encoding error.")]
    Encoding {
        #[from]
        source: std::string::FromUtf8Error,
    },
}
