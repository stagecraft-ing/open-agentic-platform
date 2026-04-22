-- Spec 110 Phase 3: Source column on factory_pipelines.
--
-- Records which trigger path produced this pipeline:
--   * 'opc-direct'  — OPC invoked the local engine directly (legacy path).
--   * 'stagecraft'  — stagecraft dispatched a factory.run.request envelope
--                     through the duplex channel (spec 110 §2.1).
--
-- Existing rows are back-filled to 'opc-direct' via the column default; all
-- runs that pre-date this spec were triggered locally.
--
-- A CHECK constraint pins the allowed values so drift in application code
-- fails at the database boundary rather than silently.

ALTER TABLE factory_pipelines
    ADD COLUMN source TEXT NOT NULL DEFAULT 'opc-direct'
        CHECK (source IN ('opc-direct', 'stagecraft'));
