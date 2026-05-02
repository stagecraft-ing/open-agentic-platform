// AgentPicker data layer (spec 126 §3, §5).
//
// TS mirror of `crates/factory-contracts/src/agent_reference.rs`.
//
// The Rust enum serialises externally-tagged (`{ "by_id": { ... } }`); this
// module uses an internal `kind` discriminant instead because the desktop has
// no shared serde→TS bridge yet. Callers that hand the reference back to the
// `factory-engine` AgentResolver MUST translate. When a generator lands, this
// declaration can be replaced with the generated form.
export type AgentReference =
  | { kind: "by_id"; org_agent_id: string; version: number }
  | { kind: "by_name"; name: string; version: number }
  | { kind: "by_name_latest"; name: string };

export type BindingStatus = "active" | "retired_upstream";

export interface BindingRow {
  binding_id: number;
  project_id: string;
  org_agent_id: string;
  pinned_version: number;
  pinned_content_hash: string;
  status: BindingStatus;
  agent_id: number | null;
  name: string | null;
  icon: string | null;
  model: string | null;
  frontmatter_json: string | null;
  created_at: string;
  updated_at: string;
}

export type CatalogStatus = "published" | "retired" | "draft";

export interface CatalogRow {
  agent_id: number;
  org_agent_id: string;
  name: string;
  icon: string;
  model: string;
  version: number;
  content_hash: string;
  status: CatalogStatus;
  frontmatter_json: string | null;
  created_at: string;
  updated_at: string;
}

export interface AgentPickerData {
  active: BindingRow[];
  browse: CatalogRow[];
  loading: boolean;
  error: Error | null;
  refresh: () => void;
}

// Stubbed in Phase 0 — body wired in Phase 2 (T020-T024).
export function useAgentPickerData(
  _orgId: string,
  _projectId?: string,
): AgentPickerData {
  return {
    active: [],
    browse: [],
    loading: false,
    error: null,
    refresh: () => {},
  };
}
