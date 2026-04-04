// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus

// NOTE (Phase 6): The stackwalk crate (tree-sitter-based code parser) has been removed as a
// dependency. do_index now uses a simplified built-in file walker that treats each source file
// as a single code block. Tree-sitter-based function-level chunking can be re-added later by
// integrating tree-sitter grammars directly into axiomregent's build.rs.

use async_trait::async_trait;
use serde_json::{json, Map, Value};
use std::sync::Arc;
use walkdir::WalkDir;

use crate::router::provider::{ToolPermissions, ToolProvider};
use super::embeddings;
use super::store::{IndexEntry, SearchStore};

/// Source file extensions supported for indexing.
const SUPPORTED_EXTENSIONS: &[&str] = &["rs", "py", "js", "ts", "go", "java", "tsx", "jsx"];

pub struct SearchProvider {
    store: Arc<SearchStore>,
}

impl SearchProvider {
    pub fn new(store: Arc<SearchStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl ToolProvider for SearchProvider {
    fn tool_schemas(&self) -> Vec<Value> {
        vec![
            json!({
                "name": "search.index",
                "description": "Index a project directory for semantic code search. Parses source files with tree-sitter, extracts code blocks, generates embeddings, and stores them for later search.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "project_name": { "type": "string", "description": "Unique name for this project index" },
                        "path": { "type": "string", "description": "Absolute path to the directory to index" }
                    },
                    "required": ["project_name", "path"]
                }
            }),
            json!({
                "name": "search.semantic",
                "description": "Search indexed code using natural language or code snippets. Returns the most semantically similar code blocks.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "project_name": { "type": "string", "description": "Project to search" },
                        "query": { "type": "string", "description": "Natural language query or code snippet" },
                        "top_k": { "type": "integer", "description": "Number of results to return (default 5)" }
                    },
                    "required": ["project_name", "query"]
                }
            }),
        ]
    }

    async fn handle(&self, name: &str, args: &Map<String, Value>) -> Option<anyhow::Result<Value>> {
        match name {
            "search.index" => {
                let project_name = match args.get("project_name").and_then(|v| v.as_str()) {
                    Some(v) => v,
                    None => return Some(Err(anyhow::anyhow!("project_name required"))),
                };
                let path = match args.get("path").and_then(|v| v.as_str()) {
                    Some(v) => v,
                    None => return Some(Err(anyhow::anyhow!("path required"))),
                };
                Some(self.do_index(project_name, path).await)
            }
            "search.semantic" => {
                let project_name = match args.get("project_name").and_then(|v| v.as_str()) {
                    Some(v) => v,
                    None => return Some(Err(anyhow::anyhow!("project_name required"))),
                };
                let query = match args.get("query").and_then(|v| v.as_str()) {
                    Some(v) => v,
                    None => return Some(Err(anyhow::anyhow!("query required"))),
                };
                let top_k = args.get("top_k").and_then(|v| v.as_u64()).unwrap_or(5) as usize;
                Some(self.do_search(project_name, query, top_k).await)
            }
            _ => None,
        }
    }

    fn tier(&self, name: &str) -> Option<agent::safety::ToolTier> {
        match name {
            "search.semantic" => Some(agent::safety::ToolTier::Tier1),
            "search.index" => Some(agent::safety::ToolTier::Tier2),
            _ => None,
        }
    }

    fn permissions(&self, name: &str) -> Option<ToolPermissions> {
        match name {
            "search.semantic" => Some(ToolPermissions {
                requires_file_read: true,
                ..Default::default()
            }),
            "search.index" => Some(ToolPermissions {
                requires_file_read: true,
                requires_file_write: true,
                ..Default::default()
            }),
            _ => None,
        }
    }
}

impl SearchProvider {
    async fn do_index(&self, project_name: &str, path: &str) -> anyhow::Result<Value> {
        // Walk the directory and collect (file_path, content) pairs for supported source files.
        // This is a simplified implementation that treats each file as one block. Tree-sitter-based
        // function-level chunking can be added back once grammars are compiled in axiomregent.
        let mut file_pairs: Vec<(String, String)> = Vec::new();
        for entry in WalkDir::new(path)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let p = entry.path();
            if !p.is_file() {
                continue;
            }
            let ext = p
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            if !SUPPORTED_EXTENSIONS.contains(&ext.as_str()) {
                continue;
            }
            // Skip files larger than 512 KiB to avoid embedding noise
            if let Ok(meta) = p.metadata() {
                if meta.len() > 512 * 1024 {
                    continue;
                }
            }
            if let Ok(content) = std::fs::read_to_string(p) {
                if !content.trim().is_empty() {
                    file_pairs.push((p.to_string_lossy().into_owned(), content));
                }
            }
        }

        if file_pairs.is_empty() {
            return Ok(json!({
                "project_name": project_name,
                "blocks_indexed": 0,
                "status": "empty"
            }));
        }

        // Generate embeddings for all file contents (CPU-bound; run off the async executor)
        let code_strings: Vec<String> = file_pairs.iter().map(|(_, c)| c.clone()).collect();
        let vectors = tokio::task::spawn_blocking(move || embeddings::embed_batch(&code_strings))
            .await??;

        // Build index entries pairing each file with its embedding
        let entries: Vec<IndexEntry> = file_pairs
            .into_iter()
            .zip(vectors.into_iter())
            .map(|((file_path, content), vector)| IndexEntry {
                file_path,
                block_type: "File".to_string(),
                function_name: None,
                code_content: content,
                vector,
                call_edges: None,
            })
            .collect();

        let count = self.store.index_project(project_name, entries).await?;

        Ok(json!({
            "project_name": project_name,
            "blocks_indexed": count,
            "status": "indexed"
        }))
    }

    async fn do_search(
        &self,
        project_name: &str,
        query: &str,
        top_k: usize,
    ) -> anyhow::Result<Value> {
        let vectors = self.store.load_vectors(project_name).await?;
        if vectors.is_empty() {
            return Ok(json!({
                "project_name": project_name,
                "results": [],
                "message": "No indexed data. Run search.index first."
            }));
        }

        let query_str = query.to_string();
        let results =
            tokio::task::spawn_blocking(move || embeddings::search_vectors(vectors, &query_str, top_k))
                .await??;

        let count = results.len();
        Ok(json!({
            "project_name": project_name,
            "results": results,
            "count": count
        }))
    }
}
