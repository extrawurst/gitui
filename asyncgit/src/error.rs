use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("`{0}`")]
    Generic(String),

    #[error("io error")]
    Io {
        #[from]
        source: std::io::Error,
    },

    #[error("git error")]
    Git {
        #[from]
        source: git2::Error,
    },
}
