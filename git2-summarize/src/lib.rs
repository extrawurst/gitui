//!	Uses Open API GPT-3 to summarize unified git diffs

use openai_api_rs::v1::{
	api::Client,
	chat_completion::{self, ChatCompletionRequest},
	common::GPT3_5_TURBO,
	completion::{self, CompletionRequest},
};

/// Uses old GPT3_TEXT_DAVINCI_003 model to generate message
///
///  # Arguments
///
/// * `api_key` - open api key
/// * `diff` - expects a diff formatted as a unified diff
pub fn git_diff_summarize_old(
	api_key: &str,
	diff: &str,
) -> Result<String, String> {
	let client = Client::new(api_key.to_string());

	let req = CompletionRequest::new(
        completion::GPT3_TEXT_DAVINCI_003.to_string(),
        format!("Generate a Git commit message based on the following summary: {}\n\nCommit message: ",diff),
    )
    .max_tokens(500)
    .temperature(0.5)
    .n(1);

	let result = client.completion(req).map_err(|e| e.message)?;
	Ok(result
		.choices
		.get(0)
		.ok_or_else(|| String::from("choises empty"))?
		.text
		.clone())
}

/// Uses GPT3_5_TURBO model to generate message using chat completion API
///
///  # Arguments
///
/// * `api_key` - open api key
/// * `diff` - expects a diff formatted as a unified diff
pub fn git_diff_summarize(
	api_key: &str,
	diff: &str,
	line_length: usize,
) -> Result<String, String> {
	let client = Client::new(api_key.to_string());

	let prompt = format!(
		r#"You are a smart git commit message creator software.
		Now you are going to create a git commit message.
		The commit messages you generate aim to explain why the changes were introduced.
		Write a one-sentence message no longer than {line_length} characters, followed by two newline characters.
		Create a commit message for these changes:\n{}
		"#,
		diff
	);

	let req = ChatCompletionRequest::new(
		GPT3_5_TURBO.to_string(),
		vec![chat_completion::ChatCompletionMessage {
			role: chat_completion::MessageRole::system,
			content: prompt,
			name: None,
			function_call: None,
		}],
	)
	.max_tokens(200);

	let result =
		client.chat_completion(req).map_err(|e| e.message)?;

	Ok(result
		.choices
		.get(0)
		.ok_or_else(|| String::from("response.choises empty"))?
		.message
		.content
		.as_ref()
		.ok_or_else(|| String::from("choise[0].message empty"))?
		.clone())
}
