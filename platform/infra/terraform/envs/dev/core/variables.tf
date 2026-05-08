variable "project_name" { type = string }
variable "location" { type = string }

variable "oidc_m2m_client_id" {
  type      = string
  sensitive = true
  default   = ""
}

variable "oidc_m2m_client_secret" {
  type      = string
  sensitive = true
  default   = ""
}

variable "stagecraft_db_url" {
  type      = string
  sensitive = true
  default   = "postgres://user:pass@host:5432/stagecraft"
}

variable "deployd_db_url" {
  type      = string
  sensitive = true
  default   = "postgres://user:pass@host:5432/deployd"
}

# Spec 143 FR-010 — per-purpose sweeper M2M client credentials.
# Each Rauthy client carries the matching `platform:<service>:sweep`
# scope in *Default Scopes* (load-bearing per §12 L-006: Rauthy 0.35
# `client_credentials` mints Default Scopes regardless of `scope=`).
# All three pairs are provisioned in Key Vault; the FU-001 beat 4
# commit only wires the knowledge pair into a CronJob — factory and
# audit are staged for FU-003 to inherit without re-deriving the
# discipline.

variable "stagecraft_knowledge_sweeper_client_id" {
  type      = string
  sensitive = true
  default   = ""
}

variable "stagecraft_knowledge_sweeper_client_secret" {
  type      = string
  sensitive = true
  default   = ""
}

variable "stagecraft_factory_sweeper_client_id" {
  type      = string
  sensitive = true
  default   = ""
}

variable "stagecraft_factory_sweeper_client_secret" {
  type      = string
  sensitive = true
  default   = ""
}

variable "stagecraft_audit_sweeper_client_id" {
  type      = string
  sensitive = true
  default   = ""
}

variable "stagecraft_audit_sweeper_client_secret" {
  type      = string
  sensitive = true
  default   = ""
}
