variable "environment" {
  type    = string
  default = "dev"
}

variable "stagecraft_host" {
  type    = string
  default = "stagecraft.localdev.online "
}

variable "deployd_host" {
  type    = string
  default = "deployd.localdev.online "
}

variable "logto_host"           { type = string }
variable "logto_admin_host"      { type = string }
variable "logto_postgres_password" { type = string }

variable "letsencrypt_email" { type = string }

variable "apply_cluster_issuer" {
  type    = bool
  default = true
}

# New custom setup variables for Logto
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
  type      = string
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
