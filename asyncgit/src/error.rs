use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("`{0}`")]
    Generic(String),

    #[error("io error:{0}")]
    Io(#[from] std::io::Error),

    #[error("git error:{0}")]
    Git(#[from] git2::Error),

    #[error("encoding error:{0}")]
    Encoding(#[from] std::string::FromUtf8Error),

    #[error("unspecified error:{0}")]
    Unspecified(Box<dyn std::error::Error>),
}

pub type Returns<T> = std::result::Result<T, Error>;

// impl From<&dyn std::error::Error> for Error {
//     fn from(error: &dyn std::error::Error) -> Self {
//         let e = error.clone().to_owned();
//         let b = Box::new(e);
//         Error::Unspecified(b)
//     }
// }

impl<T> From<std::sync::PoisonError<T>> for Error {
    fn from(error: std::sync::PoisonError<T>) -> Self {
        Error::Generic(format!("poison error: {}", error))
    }
}
