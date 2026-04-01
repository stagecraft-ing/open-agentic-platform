-- Seed a system user for M2M audit records from OPC axiomregent.
-- The audit_log.actor_user_id column is NOT NULL FK → users(id),
-- so platform-originated entries need a well-known system actor.
INSERT INTO users (id, name, email, password_hash, role)
VALUES (
  '00000000-0000-0000-0000-000000000000',
  'system',
  'system@opc.local',
  '!disabled',
  'admin'
)
ON CONFLICT (id) DO NOTHING;
