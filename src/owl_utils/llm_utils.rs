use crate::{common::OwlError, owl_utils::toml_utils};
use anthropic_sdk::{Anthropic, ContentBlock, MessageCreateBuilder};
use std::path::Path;

#[derive(Debug, PartialEq)]
pub enum PromptMode {
    Custom,
    Debug,
    Default,
    Explain,
    Explore,
    Optimize,
    Test,
}

const DEBUG_PROMPT: &str = r#"
Here's a piece of code that isn't passing the tests:
[paste]
Consider:
1. Potential bugs or edge cases
2. Performance optimizations
Suggest improvements and explain your reasoning for each suggestion.
"#;

const DESC_PROMPT: &str = r#"
Please review the following problem description:
[paste]
I'm trying to implement a program to solve this problem.
"#;

const DEFAULT_PROMPT: &str = r#"
Please review the following code:
[paste]
Consider:
1. Code quality and adherence to best practices
2. Potential bugs or edge cases
3. Performance optimizations
4. Readability and maintainability
5. Any security concerns
Suggest improvements and explain your reasoning for each suggestion.
"#;

const EXPLAIN_PROMPT: &str = r#"
This is the program that I have implemented so far.
[paste]
I do not understand this problem that I have been trying to solve. Could you please explain the problem?
Consider:
1. Important concepts and constraints
2. Potential edge cases
3. Notable data structures and algorithms
Please do not solve the problem for me or provide me with code.
"#;

const EXPLORE_PROMPT: &str = r#"
Please review the following code:
[paste]
Consider:
1. Code quality and adherence to best practices
2. Readability and maintainability
3. Other methods, libraries, or packages
Suggest improvements and explain your reasoning for each suggestion.
"#;

const OPT_PROMPT: &str = r#"
Here's a piece of code that needs optimization:
[paste]
Please suggest optimizations to improve its performance. For each suggestion, explain the expected improvement and any trade-offs.
"#;

const PLACEHOLDER: &str = "[paste]";

const TEST_PROMPT: &str = r#"
Could you suggest test cases for the following program:
[paste]
Include tests for:
1. Normal expected inputs
2. Edge cases
All inputs will be valid. Please explain your reasoning for each suggestion.
"#;

pub async fn llm_review(
    manifest_path: &Path,
    prog_str: &str,
    mode: PromptMode,
    prompt_str: Option<String>,
) -> Result<(String, String), OwlError> {
    let (ai_sdk, api_key) = toml_utils::get_manifest_ai_sdk(manifest_path)?;

    if ai_sdk.is_empty() {
        return Err(OwlError::LlmError(
            "no LLM has been selected".into(),
            "".into(),
        ));
    }

    if api_key.is_empty() {
        return Err(OwlError::LlmError(
            "no API key has been provided".into(),
            "".into(),
        ));
    }

    match ai_sdk.as_str() {
        "claude" => println!("Sending code review to {}...", ai_sdk),
        _ => {
            return Err(OwlError::Unsupported(format!(
                "'{}': not supported",
                ai_sdk
            )));
        }
    };

    let client = Anthropic::new(api_key).map_err(|e| {
        OwlError::LlmError(format!("could not connect to '{}'", ai_sdk), e.to_string())
    })?;

    let suggested_prompt = match mode {
        PromptMode::Debug => DEBUG_PROMPT.replace(PLACEHOLDER, prog_str),
        PromptMode::Explain => EXPLAIN_PROMPT.replace(PLACEHOLDER, prog_str),
        PromptMode::Explore => EXPLORE_PROMPT.replace(PLACEHOLDER, prog_str),
        PromptMode::Optimize => OPT_PROMPT.replace(PLACEHOLDER, prog_str),
        PromptMode::Test => TEST_PROMPT.replace(PLACEHOLDER, prog_str),
        _ => DEFAULT_PROMPT.replace(PLACEHOLDER, prog_str),
    };

    let user_prompt = if let Some(text) = prompt_str {
        if mode == PromptMode::Custom {
            if text.contains(PLACEHOLDER) {
                text.replace(PLACEHOLDER, prog_str)
            } else {
                format!(
                    "Hello! Please review the following code: {}\n{}",
                    prog_str, text
                )
            }
        } else {
            format!(
                "{}\n{}",
                DESC_PROMPT.replace(PLACEHOLDER, &text),
                suggested_prompt
            )
        }
    } else {
        suggested_prompt
    };

    let response = client
        .messages()
        .create(
            MessageCreateBuilder::new("claude-sonnet-4-5", 1024)
                .user(user_prompt)
                .build(),
        )
        .await
        .map_err(|e| {
            OwlError::LlmError(
                format!("could not send prompt to '{}'", ai_sdk),
                e.to_string(),
            )
        })?;

    let mut buffer = String::new();
    for content_block in response.content {
        if let ContentBlock::Text { text } = content_block {
            buffer.push_str(&format!("\n{}: ", ai_sdk));
            buffer.push_str(&text);
        }
    }

    Ok((ai_sdk, buffer))
}
