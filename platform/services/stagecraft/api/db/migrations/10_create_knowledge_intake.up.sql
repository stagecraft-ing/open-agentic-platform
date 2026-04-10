-- Knowledge Intake Domain (spec 087 Phase 2)
-- Adds source_connectors, knowledge_objects, and document_bindings tables.

-- ---------------------------------------------------------------------------
-- Enums
-- ---------------------------------------------------------------------------

CREATE TYPE connector_type AS ENUM (
    'upload', 'sharepoint', 's3', 'azure-blob', 'gcs'
);

CREATE TYPE connector_status AS ENUM (
    'active', 'paused', 'error', 'disabled'
);

CREATE TYPE knowledge_object_state AS ENUM (
    'imported', 'extracting', 'extracted', 'classified', 'available'
);

-- ---------------------------------------------------------------------------
-- Source connectors: external knowledge source configuration
-- ---------------------------------------------------------------------------

CREATE TABLE source_connectors (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id),
    type connector_type NOT NULL,
    name TEXT NOT NULL,
    config_encrypted JSONB,
    sync_schedule TEXT,
    status connector_status NOT NULL DEFAULT 'active',
    last_synced_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_source_connectors_workspace ON source_connectors(workspace_id);

-- ---------------------------------------------------------------------------
-- Knowledge objects: canonical normalised documents in workspace store
-- ---------------------------------------------------------------------------

CREATE TABLE knowledge_objects (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id),
    connector_id UUID REFERENCES source_connectors(id),
    storage_key TEXT NOT NULL,
    filename TEXT NOT NULL,
    mime_type TEXT NOT NULL,
    size_bytes BIGINT NOT NULL,
    content_hash TEXT NOT NULL,
    state knowledge_object_state NOT NULL DEFAULT 'imported',
    extraction_output JSONB,
    classification JSONB,
    provenance JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_knowledge_objects_workspace ON knowledge_objects(workspace_id);
CREATE INDEX idx_knowledge_objects_state ON knowledge_objects(workspace_id, state);
CREATE INDEX idx_knowledge_objects_connector ON knowledge_objects(connector_id);

-- ---------------------------------------------------------------------------
-- Document bindings: links knowledge objects to projects
-- ---------------------------------------------------------------------------

CREATE TABLE document_bindings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id),
    knowledge_object_id UUID NOT NULL REFERENCES knowledge_objects(id),
    bound_by UUID NOT NULL REFERENCES users(id),
    bound_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(project_id, knowledge_object_id)
);

CREATE INDEX idx_document_bindings_project ON document_bindings(project_id);
CREATE INDEX idx_document_bindings_knowledge ON document_bindings(knowledge_object_id);
