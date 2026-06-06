use crate::models::message::Message;
use std::collections::HashSet;

/// Token estimation utilities.
pub struct TokenCounter;

impl TokenCounter {
    /// Rough token estimate: ~4 chars per token
    pub fn estimate(text: &str) -> usize {
        (text.len() / 4).max(1)
    }

    /// Estimate tokens including per-message overhead (~5 tokens for role tags)
    pub fn estimate_message(msg: &Message) -> usize {
        5 + Self::estimate(&msg.content)
    }

    /// Estimate total tokens for a conversation
    pub fn estimate_conversation(conv: &[Message]) -> usize {
        conv.iter().map(|m| Self::estimate_message(m)).sum()
    }
}

/// Configuration for context compression.
#[derive(Debug, Clone)]
pub struct CompressionConfig {
    pub enabled: bool,
    /// Fraction of context to target (0.0–1.0)
    pub target_usage: f64,
    /// Minimum recent messages to always keep
    pub min_recent_messages: usize,
    /// Max chars for a single tool output before truncation
    pub max_tool_output_chars: usize,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            target_usage: 0.80,
            min_recent_messages: 8,
            max_tool_output_chars: 2000,
        }
    }
}

/// Context compressor implementing two-phase prune + compact.
pub struct ContextCompressor {
    config: CompressionConfig,
}

impl ContextCompressor {
    pub fn new(config: CompressionConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &CompressionConfig {
        &self.config
    }

    /// Compress a conversation to fit within the target token budget.
    /// Returns (compressed conversation, summary of what was removed).
    pub fn compress(&self, conversation: &[Message], context_size: usize) -> (Vec<Message>, String) {
        if !self.config.enabled || conversation.is_empty() {
            return (conversation.to_vec(), String::new());
        }

        let token_budget = (context_size as f64 * self.config.target_usage) as usize;
        let char_budget = token_budget * 4;

        // Phase 1: Prune oversized tool outputs
        let pruned = self.prune_tool_outputs(conversation);

        // Quick check: already within budget
        let current_chars: usize = pruned.iter().map(|m| m.content.len() + 50).sum();
        if current_chars <= char_budget {
            return (pruned, String::new());
        }

        // Phase 2: Compact
        self.compact(pruned, char_budget)
    }

    fn prune_tool_outputs(&self, conversation: &[Message]) -> Vec<Message> {
        let max_chars = self.config.max_tool_output_chars;
        conversation.iter().map(|msg| {
            if msg.role == "tool" && msg.content.len() > max_chars {
                let mut m = msg.clone();
                m.content = format!(
                    "{}... [truncated {} chars]",
                    &msg.content[..max_chars],
                    msg.content.len() - max_chars
                );
                m
            } else {
                msg.clone()
            }
        }).collect()
    }

    fn compact(&self, conversation: Vec<Message>, char_budget: usize) -> (Vec<Message>, String) {
        let total = conversation.len();
        let mut summary = String::new();
        let mut keep: Vec<(usize, Message)> = Vec::new();
        let mut kept_indices: HashSet<usize> = HashSet::new();

        // 1. Always keep system prompt (index 0)
        if let Some(sys) = conversation.first() {
            keep.push((0, sys.clone()));
            kept_indices.insert(0);
        }

        // 2. Always keep user's first task message (index 1)
        if conversation.len() > 1 && conversation[1].role == "user" {
            keep.push((1, conversation[1].clone()));
            kept_indices.insert(1);
        }

        let base_cost: usize = keep.iter().map(|(_, m)| m.content.len() + 50).sum();
        let mut budget_remaining = char_budget.saturating_sub(base_cost);

        // 3. Add remaining messages from most recent to oldest
        for i in (2..total).rev() {
            if kept_indices.contains(&i) {
                continue;
            }

            let msg = &conversation[i];
            let is_recent = i >= total.saturating_sub(self.config.min_recent_messages);
            let msg_cost = msg.content.len() + 50;

            if is_recent || budget_remaining >= msg_cost {
                keep.push((i, msg.clone()));
                kept_indices.insert(i);
                if !is_recent {
                    budget_remaining = budget_remaining.saturating_sub(msg_cost);
                }
            } else {
                // Track removed messages for summary
                let snippet: String = msg.content.chars().take(80).collect();
                if !summary.is_empty() {
                    summary.push('\n');
                }
                summary.push_str(&format!("[{}: {}...]", msg.role, snippet));
            }
        }

        // Sort by original index to restore chronological order
        keep.sort_by_key(|(i, _)| *i);

        // Insert compression marker if messages were removed
        let result: Vec<Message> = if summary.len() > 10 {
            let marker = Message::system(
                "[Earlier messages compressed for context efficiency. Summary available in memory.]"
            );
            let insert_at = if keep.len() > 2 && keep[1].1.role == "user" { 2 } else { 1 };
            let mut result: Vec<Message> = keep.into_iter().map(|(_, m)| m).collect();
            result.insert(insert_at, marker);
            result
        } else {
            keep.into_iter().map(|(_, m)| m).collect()
        };

        (result, summary)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_msg(role: &str, content: &str) -> Message {
        match role {
            "system" => Message::system(content),
            "user" => Message::user(content),
            "assistant" => Message::assistant(content),
            "tool" => Message::tool(content, "call_comp_test"),
            _ => Message::user(content),
        }
    }

    #[test]
    fn test_token_estimate() {
        let n = TokenCounter::estimate("hello world");
        assert!(n >= 1);
    }

    #[test]
    fn test_token_estimate_message() {
        let msg = make_msg("user", "hello world");
        let n = TokenCounter::estimate_message(&msg);
        assert!(n >= 6);
    }

    #[test]
    fn test_compressor_disabled() {
        let c = ContextCompressor::new(CompressionConfig {
            enabled: false,
            ..Default::default()
        });
        let conv = vec![make_msg("system", "sys"), make_msg("user", "hi")];
        let (result, _) = c.compress(&conv, 4096);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_prune_tool_output() {
        let c = ContextCompressor::new(CompressionConfig {
            max_tool_output_chars: 10,
            ..Default::default()
        });
        let long = "x".repeat(100);
        let conv = vec![
            make_msg("system", "sys"),
            make_msg("user", "hi"),
            make_msg("tool", &long),
        ];
        let (result, _) = c.compress(&conv, 4096);
        let tool_msgs: Vec<_> = result.iter().filter(|m| m.role == "tool").collect();
        assert_eq!(tool_msgs.len(), 1);
        assert!(tool_msgs[0].content.len() < 50);
        assert!(tool_msgs[0].content.contains("truncated"));
    }

    #[test]
    fn test_small_conversation_stays_unchanged() {
        let c = ContextCompressor::new(CompressionConfig::default());
        let conv = vec![
            make_msg("system", "sys prompt"),
            make_msg("user", "do something"),
            make_msg("assistant", "ok"),
        ];
        let (result, _) = c.compress(&conv, 4096);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].content, "sys prompt");
    }

    #[test]
    fn test_large_conversation_gets_compressed() {
        let c = ContextCompressor::new(CompressionConfig {
            min_recent_messages: 2,
            ..Default::default()
        });
        let mut conv = vec![make_msg("system", "sys"), make_msg("user", "task")];
        for i in 0..20 {
            conv.push(make_msg("assistant", &format!("response {}", i)));
            conv.push(make_msg("tool", &format!("result {}", i)));
        }
        let (result, _) = c.compress(&conv, 256);
        assert!(result.len() < conv.len());
        assert_eq!(result[0].role, "system");
        assert!(result.iter().any(|m| m.role == "user"));
    }

    #[test]
    fn test_compression_keeps_recent_messages() {
        let c = ContextCompressor::new(CompressionConfig {
            min_recent_messages: 4,
            ..Default::default()
        });
        let mut conv = vec![make_msg("system", "sys"), make_msg("user", "task")];
        for i in 0..10 {
            conv.push(make_msg("assistant", &format!("resp{}", i)));
            conv.push(make_msg("tool", &format!("res{}", i)));
        }
        let total = conv.len();
        let (result, _) = c.compress(&conv, 256);
        // The last min_recent_messages (4) messages should be present
        let last_few: Vec<&str> = conv.iter().skip(total - 4).map(|m| m.content.as_str()).collect();
        for content in &last_few {
            assert!(result.iter().any(|m| &m.content == content),
                "missing recent message: {}", content);
        }
    }
}
