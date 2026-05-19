//! # TENET AI Agent (rig.rs)
//!
//! Builds and runs the TENET AI agent using the `rig` framework.
//! This implementation enforces STRICT JSON output to bypass Groq provider errors.

use anyhow::{anyhow, Result};
use rig::{completion::Prompt, providers::{anthropic, gemini, openai}};
use serde::Deserialize;
use std::path::PathBuf;

use crate::tools::{
    DiffVersionsArgs, DiffVersionsTool, GetHistoryArgs, GetHistoryTool, ListFilesArgs,
    ListFilesTool, RestoreVersionArgs, RestoreVersionTool,
};
use rig::tool::Tool;

// ─── System Prompt ────────────────────────────────────────────────────────────

const SYSTEM_PROMPT: &str = r#"You are TENET AI assistant.

CRITICAL RULES:
- You MUST ONLY output valid JSON.
- DO NOT output XML, HTML, or custom tags.
- DO NOT use <function=...> format.
- ALWAYS follow this schema EXACTLY:

{
  "tool": "tool_name",
  "arguments": {
    "key": "value"
  }
}

Available tools:
- get_history(file: string)
- restore_version(file: string, time: string)
- list_files()
- diff_versions(file: string, v1: string, v2: string)

If no tool is needed or you just want to answer a question, return:
{
  "tool": "none",
  "arguments": {
    "message": "your text response"
  }
}

Time format for restore_version:
- For relative time: use "1h", "30m", "2d", etc.
- For version numbers: pass "v1", "v2", "1", "2" etc. directly. The tool handles version lookups automatically.
- You do NOT need to call get_history first to resolve a version number.
"#;

#[derive(Deserialize, Debug)]
struct ToolCall {
    tool: String,
    arguments: serde_json::Value,
}

// ─── Agent entry point ────────────────────────────────────────────────────────

pub async fn run_agent(
    query: &str,
    watched_dir: PathBuf,
    tenet_dir: PathBuf,
    api_key: &str,
    provider: &str,
) -> Result<String> {
    let mut current_query = query.to_string();

    // We do a retry loop in case the LLM fails to output valid JSON
    for attempt in 1..=2 {
        let response_text = match provider {
            "groq" => {
                let client = openai::Client::from_url(api_key, "https://api.groq.com/openai/v1");
                let agent = client.agent("llama-3.3-70b-versatile").temperature(0.0).preamble(SYSTEM_PROMPT).build();
                agent.prompt(current_query.as_str()).await?
            }
            "openai" => {
                let client = openai::Client::new(api_key);
                let agent = client.agent("gpt-4o-mini").temperature(0.0).preamble(SYSTEM_PROMPT).build();
                agent.prompt(current_query.as_str()).await?
            }
            "anthropic" => {
                let client = anthropic::Client::new(api_key, "https://api.anthropic.com/v1", None, "2023-06-01");
                let agent = client.agent("claude-3-5-sonnet-20241022").temperature(0.0).preamble(SYSTEM_PROMPT).build();
                agent.prompt(current_query.as_str()).await?
            }
            "gemini" => {
                let client = gemini::Client::new(api_key);
                let agent = client.agent("gemini-2.0-flash").temperature(0.0).preamble(SYSTEM_PROMPT).build();
                agent.prompt(current_query.as_str()).await?
            }
            _ => return Err(anyhow!("Unknown provider: {}", provider)),
        };
        
        // Try to parse the response as JSON
        let json_start = response_text.find('{');
        let json_end = response_text.rfind('}');
        
        if let (Some(start), Some(end)) = (json_start, json_end) {
            let json_str = &response_text[start..=end];
            match serde_json::from_str::<ToolCall>(json_str) {
                Ok(tool_call) => {
                    return execute_tool(tool_call, watched_dir, tenet_dir).await;
                }
                Err(e) => {
                    if attempt == 2 {
                        return Err(anyhow!("Failed to parse JSON tool call: {}", e));
                    }
                    current_query = format!("Your previous output was invalid JSON. You MUST return valid JSON only. Error: {}. Original Query: {}", e, query);
                    continue;
                }
            }
        } else {
            if attempt == 2 {
                return Err(anyhow!("No JSON object found in model output: {}", response_text));
            }
            current_query = format!("Your previous output did not contain a JSON object. You MUST return valid JSON only. Original Query: {}", query);
        }
    }

    Err(anyhow!("Agent failed to output valid JSON after retries."))
}

async fn execute_tool(
    call: ToolCall,
    watched_dir: PathBuf,
    tenet_dir: PathBuf,
) -> Result<String> {
    match call.tool.as_str() {
        "get_history" => {
            let args: GetHistoryArgs = serde_json::from_value(call.arguments)?;
            let tool = GetHistoryTool { watched_dir, tenet_dir };
            tool.call(args).await.map_err(|e| anyhow!(e.0))
        }
        "restore_version" => {
            let args: RestoreVersionArgs = serde_json::from_value(call.arguments)?;
            let tool = RestoreVersionTool { watched_dir, tenet_dir };
            tool.call(args).await.map_err(|e| anyhow!(e.0))
        }
        "list_files" => {
            let args: ListFilesArgs = serde_json::from_value(call.arguments)?;
            let tool = ListFilesTool { watched_dir, tenet_dir };
            tool.call(args).await.map_err(|e| anyhow!(e.0))
        }
        "diff_versions" => {
            let args: DiffVersionsArgs = serde_json::from_value(call.arguments)?;
            let tool = DiffVersionsTool { watched_dir, tenet_dir };
            tool.call(args).await.map_err(|e| anyhow!(e.0))
        }
        "none" => {
            // Just return the message the LLM generated
            let msg = call.arguments.get("message").and_then(|v| v.as_str()).unwrap_or("No action taken.");
            Ok(msg.to_string())
        }
        other => Err(anyhow!("LLM hallucinated unknown tool: {}", other)),
    }
}
