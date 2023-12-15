use std::env;

fn main() {
	let diff = include_str!("simple.diff");

	let summary = git2_summarize::git_diff_summarize_old(
		&env::var("OPENAI_API_KEY").unwrap(),
		diff,
	)
	.unwrap();

	println!("{summary}");
}
