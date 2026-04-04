// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus

use async_trait::async_trait;
use serde_json::{json, Map, Value};
use std::sync::Arc;

use crate::router::provider::{ToolPermissions, ToolProvider};
use super::embeddings;
use super::store::{IndexEntry, SearchStore};

/// Embedded asterisk.toml config — avoids fragile include_str! relative paths
/// across crate boundaries. Keep in sync with crates/stackwalk/asterisk.toml.
const ASTERISK_TOML: &str = r#"
[languages]
  [languages.python]
    [languages.python.matchers]
      import_statement = "import_from_statement"
      [languages.python.matchers.module_name]
        field_name = "module_name"
        kind = "dotted_name"

      [languages.python.matchers.object_name]
        field_name = "name"
        kind = "dotted_name"

      [languages.python.matchers.alias]
        field_name = "alias"
        kind = "identifier"

  [languages.rust]
    [languages.rust.matchers]
      import_statement = "use_declaration"
      [languages.rust.matchers.module_name]
        field_name = "path"
        kind = "identifier"

      [languages.rust.matchers.object_name]
        field_name = "name"
        kind = "identifier"

      [languages.rust.matchers.alias]
        field_name = "alias"
        kind = "identifier"
"#;

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
        let config = stackwalk::config::Config::from_toml(ASTERISK_TOML)
            .map_err(|e| anyhow::anyhow!("Config parse error: {}", e))?;

        let (blocks, _call_stack, _call_graph) =
            stackwalk::indexer::index_directory(&config, path);

        if blocks.is_empty() {
            return Ok(json!({
                "project_name": project_name,
                "blocks_indexed": 0,
                "status": "empty"
            }));
        }

        // Generate embeddings for all code blocks (CPU-bound; run off the async executor)
        let code_strings: Vec<String> = blocks.iter().map(|b| b.content.clone()).collect();
        let vectors = tokio::task::spawn_blocking(move || embeddings::embed_batch(&code_strings))
            .await??;

        // Build index entries pairing each block with its embedding
        let entries: Vec<IndexEntry> = blocks
            .iter()
            .zip(vectors.into_iter())
            .map(|(block, vector)| {
                // node_key is "<file_path>.<class?>.<function>" — the file path is the prefix
                // before the first dot that is a path separator; use the raw node_key as
                // file_path so callers can correlate back to the source file.
                let file_path = block
                    .node_key
                    .split('.')
                    .next()
                    .unwrap_or(&block.node_key)
                    .to_string();

                let call_edges = if block.outgoing_calls.is_empty() {
                    None
                } else {
                    serde_json::to_string(&block.outgoing_calls).ok()
                };

                IndexEntry {
                    file_path,
                    block_type: format!("{:?}", block.block_type),
                    function_name: block.function_name.clone(),
                    code_content: block.content.clone(),
                    vector,
                    call_edges,
                }
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
