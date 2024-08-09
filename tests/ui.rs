use std::path::Path;

use snapbox::{cmd::Command, data::DataFormat, Data};
use tempfile::TempDir;

#[test]
fn test_empty_dir() {
	let path: &Path = Path::new("tests/fixtures/empty_dir.svg");

	let empty_dir = TempDir::new().unwrap();

	Command::new("gitui")
		.current_dir(empty_dir.path())
		.assert()
		.success()
		.stderr_eq(Data::read_from(path, Some(DataFormat::TermSvg)));
}
