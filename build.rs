fn get_git_hash() -> String {
	use std::process::Command;

	let commit = Command::new("git")
		.arg("rev-parse")
		.arg("--short")
		.arg("--verify")
		.arg("HEAD")
		.output();
	if let Ok(commit_output) = commit {
		let commit_string =
			String::from_utf8_lossy(&commit_output.stdout);

		return commit_string.lines().next().unwrap_or("").into();
	}

	panic!("Can not get git commit: {}", commit.unwrap_err());
}

fn main() {
	let build_name = if std::env::var("GITUI_RELEASE").is_ok() {
		format!(
			"{} {} ({})",
			env!("CARGO_PKG_VERSION"),
			compile_time::date_str!(),
			get_git_hash()
		)
	} else {
		format!(
			"nightly {} ({})",
			compile_time::date_str!(),
			get_git_hash()
		)
	};

	println!("cargo:warning=buildname '{}'", build_name);
	println!("cargo:rustc-env=GITUI_BUILD_NAME={}", build_name);
}
