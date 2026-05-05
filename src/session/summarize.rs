use anyhow::{Context, Result};

use super::normalize::ParsedSession;
use crate::config::Config;

/// Summarize a parsed session for ingestion into Graphiti.
pub async fn summarize(session: &ParsedSession, no_llm: bool) -> Result<String> {
    if no_llm {
        return Ok(extractive_summary(session));
    }

    match llm_summary(session).await {
        Ok(summary) => Ok(summary),
        Err(e) => {
            eprintln!(
                "  LLM summarization failed ({}), using extractive fallback",
                e
            );
            Ok(extractive_summary(session))
        }
    }
}

fn extractive_summary(session: &ParsedSession) -> String {
    let mut parts = Vec::new();

    if let Some(first_turn) = session.turns.first()
        && let Some(ref msg) = first_turn.user_message
    {
        let prompt = truncate_str(msg, 300);
        parts.push(format!("Initial prompt: {prompt}"));
    }

    if !session.files_touched.is_empty() {
        let files: Vec<&str> = session.files_touched.iter().take(20).map(|s| s.as_str()).collect();
        parts.push(format!("Files: {}", files.join(", ")));
    }

    if !session.tools_used.is_empty() {
        parts.push(format!("Tools: {}", session.tools_used.join(", ")));
    }

    parts.push(format!("Turns: {}", session.turns.len()));

    if let Some(ref m) = session.model {
        parts.push(format!("Model: {m}"));
    }

    let duration = session.ended_at - session.started_at;
    let mins = duration.num_minutes();
    if mins > 0 {
        parts.push(format!("Duration: {mins}m"));
    }

    parts.join("\n")
}

async fn llm_summary(session: &ParsedSession) -> Result<String> {
    let config = Config::load()?;
    let api_url = config.graphiti.llm.api_url.trim_end_matches("/v1");
    let api_url = format!("{api_url}/api/chat");

    let conversation = format_conversation(session);

    let prompt = format!(
        "Summarize this AI coding session in 2-3 concise sentences. \
         Focus on what was accomplished, key decisions made, and the outcome.\n\n\
         Agent: {agent}\n\
         Project: {origin}\n\
         Model: {model}\n\
         Turns: {turns}\n\
         Files touched: {files}\n\n\
         Conversation:\n{conversation}",
        agent = session.agent,
        origin = session.origin,
        model = session.model.as_deref().unwrap_or("unknown"),
        turns = session.turns.len(),
        files = session.files_touched.join(", "),
    );

    let body = serde_json::json!({
        "model": config.graphiti.llm.model,
        "messages": [
            {"role": "user", "content": prompt}
        ],
        "stream": false
    });

    let client = reqwest::Client::new();
    let resp = client
        .post(&api_url)
        .json(&body)
        .timeout(std::time::Duration::from_secs(60))
        .send()
        .await
        .context("failed to reach Ollama")?;

    if !resp.status().is_success() {
        anyhow::bail!("Ollama returned {}", resp.status());
    }

    let json: serde_json::Value = resp.json().await?;
    let content = json["message"]["content"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_string();

    if content.is_empty() {
        anyhow::bail!("empty response from Ollama");
    }

    Ok(content)
}

fn format_conversation(session: &ParsedSession) -> String {
    let mut output = String::new();
    let max_chars = 4000;

    // First turn + last 3 turns (with dedup)
    let turn_count = session.turns.len();
    let selected: Vec<usize> = if turn_count <= 4 {
        (0..turn_count).collect()
    } else {
        let mut indices = vec![0];
        for i in turn_count.saturating_sub(3)..turn_count {
            if i != 0 {
                indices.push(i);
            }
        }
        indices
    };

    for (idx, &i) in selected.iter().enumerate() {
        let turn = &session.turns[i];

        if idx > 0 && selected[idx] != selected[idx - 1] + 1 {
            output.push_str(&format!("\n[... {} turns omitted ...]\n\n", selected[idx] - selected[idx - 1] - 1));
        }

        if let Some(ref msg) = turn.user_message {
            output.push_str(&format!("User: {}\n", truncate_str(msg, 300)));
        }
        if !turn.assistant_message.is_empty() {
            output.push_str(&format!(
                "Assistant: {}\n",
                truncate_str(&turn.assistant_message, 500)
            ));
        }
        for tc in &turn.tool_calls {
            output.push_str(&format!("  [{}] {}\n", tc.name, tc.input_summary));
        }
        output.push('\n');

        if output.len() > max_chars {
            output.truncate(max_chars);
            output.push_str("\n[truncated]");
            break;
        }
    }

    output
}

fn truncate_str(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        let end = s
            .char_indices()
            .nth(max)
            .map(|(i, _)| i)
            .unwrap_or(s.len());
        format!("{}...", &s[..end])
    }
}
