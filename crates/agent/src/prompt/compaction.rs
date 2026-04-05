// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
// Feature: 070-PROMPT_ASSEMBLY_CACHE

//! Context compaction service (FR-006, integrates with spec 046).
//!
//! Monitors conversation size against the model's context window and triggers
//! summarization when the threshold is reached. Always keeps the last K messages
//! uncompacted to preserve recent context (R-002 mitigation).

/// A conversation message (role + content) for size accounting.
#[derive(Debug, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
}

impl Message {
    /// Approximate byte size (role + content).
    pub fn byte_size(&self) -> usize {
        self.role.len() + self.content.len()
    }
}

/// Output of a compaction pass.
#[derive(Debug, Clone)]
pub struct CompactedResult {
    /// Summary of the compacted messages.
    pub summary: String,
    /// Recent messages preserved verbatim.
    pub kept_messages: Vec<Message>,
    /// How many messages were summarized.
    pub compacted_count: usize,
}

/// Compaction service that monitors context size and triggers summarization (FR-006).
pub struct CompactionService {
    /// Fraction of the context window that triggers compaction (default 0.8).
    pub window_threshold: f32,
    /// Number of recent messages to keep uncompacted (default 10).
    pub keep_recent: usize,
    /// Pluggable summarizer — takes a slice of messages, returns a summary string.
    pub summarizer: Box<dyn Fn(&[Message]) -> String + Send + Sync>,
}

impl CompactionService {
    /// Create a compaction service with defaults and the given summarizer.
    pub fn new(summarizer: impl Fn(&[Message]) -> String + Send + Sync + 'static) -> Self {
        Self {
            window_threshold: 0.8,
            keep_recent: 10,
            summarizer: Box::new(summarizer),
        }
    }

    /// Check whether compaction should fire (FR-006).
    ///
    /// Returns `true` when `prompt_size + message_size >= threshold * model_context_window`.
    pub fn should_compact(
        &self,
        prompt_size: usize,
        messages: &[Message],
        model_context_window: usize,
    ) -> bool {
        let message_size: usize = messages.iter().map(|m| m.byte_size()).sum();
        let total = prompt_size + message_size;
        let limit = (model_context_window as f64 * self.window_threshold as f64) as usize;
        total >= limit
    }

    /// Run compaction: summarize older messages, keep recent ones (R-002).
    pub fn compact(&self, messages: &[Message]) -> CompactedResult {
        if messages.len() <= self.keep_recent {
            // Nothing to compact — all messages are "recent".
            return CompactedResult {
                summary: String::new(),
                kept_messages: messages.to_vec(),
                compacted_count: 0,
            };
        }

        let split = messages.len() - self.keep_recent;
        let to_summarize = &messages[..split];
        let to_keep = &messages[split..];

        let summary = (self.summarizer)(to_summarize);

        CompactedResult {
            summary,
            kept_messages: to_keep.to_vec(),
            compacted_count: split,
        }
    }
}
