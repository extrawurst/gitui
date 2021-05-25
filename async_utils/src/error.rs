use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("`{0}`")]
    Generic(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl<T> From<crossbeam_channel::SendError<T>> for Error {
    fn from(error: crossbeam_channel::SendError<T>) -> Self {
        Self::Generic(format!("send error: {}", error))
    }
}

impl<T> From<std::sync::PoisonError<T>> for Error {
    fn from(error: std::sync::PoisonError<T>) -> Self {
        Self::Generic(format!("poison error: {}", error))
    }
}
