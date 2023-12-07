#![allow(renamed_and_removed_lints, clippy::unknown_clippy_lints)]

use thiserror::Error;

///
#[derive(Error, Debug)]
pub enum Error {
	///
	#[error("`{0}`")]
	Generic(String),

	///
	#[error("git error:{0}")]
	Git(#[from] git2::Error),

	///
	#[error("io error:{0}")]
	Io(#[from] std::io::Error),

	///
	#[error("path string conversion error")]
	PathToString,

	///
	#[error("shellexpand error:{0}")]
	Shell(#[from] shellexpand::LookupError<std::env::VarError>),
}

///
pub type Result<T> = std::result::Result<T, Error>;
