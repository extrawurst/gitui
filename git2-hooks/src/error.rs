use thiserror::Error;

///
#[derive(Error, Debug)]
pub enum HooksError {
	///
	#[error("io error:{0}")]
	Io(#[from] std::io::Error),

	///
	#[error("path string conversion error")]
	PathToString,

	///
	#[error("shellexpand error:{0}")]
	ShellExpand(#[from] shellexpand::LookupError<std::env::VarError>),
}

///
pub type Result<T> = std::result::Result<T, HooksError>;
