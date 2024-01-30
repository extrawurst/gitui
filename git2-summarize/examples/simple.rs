use std::env;

fn main() {
	let diff = include_str!("simple.diff");

	let summary = git2_summarize::git_diff_summarize(
		&env::var("OPENAI_API_KEY").unwrap(),
		diff,
		50,
	)
	.unwrap();

	println!("{summary}");
}
