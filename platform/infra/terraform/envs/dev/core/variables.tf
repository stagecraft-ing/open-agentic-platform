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
