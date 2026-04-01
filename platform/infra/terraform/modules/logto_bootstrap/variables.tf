variable "charts_root" { type = string }
variable "namespace" { type = string }

variable "logto_host" { type = string } # eg logto.stagecraft.ing
variable "admin_host" { type = string } # eg logto-admin.stagecraft.ing

variable "postgres_password" {
  type      = string
  sensitive = true
}

# New custom setup variables
variable "domain" { type = string }
variable "logto_spa_client_id" { type = string }
variable "logto_spa_api_resource" { type = string }
variable "logto_m2m_client_id" { type = string }
variable "app_name" { type = string }
variable "app_url" { type = string }

variable "logto_spa_client_secret" {
  type      = string
  sensitive = true
}
variable "logto_spa_api_event_webhook_url" {
  type = string
}

variable "logto_m2m_client_secret" {
  type      = string
  sensitive = true
}
variable "logto_spa_api_event_webhook_signing_key" {
  type      = string
  sensitive = true
}
variable "logto_google_connector_id" { type = string }
variable "logto_google_client_id" { type = string }
variable "logto_google_client_secret" {
  type      = string
  sensitive = true
}
variable "logto_google_workspace_connector_id" { type = string }
variable "logto_google_workspace_client_id" { type = string }
variable "logto_google_workspace_client_secret" {
  type      = string
  sensitive = true
}
variable "logto_google_workspace_connector_approved_domains" { type = string }
