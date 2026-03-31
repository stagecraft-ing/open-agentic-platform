variable "ingress_nginx_enabled" { type = bool }
variable "cert_manager_enabled"  { type = bool }
variable "csi_secrets_enabled"   { type = bool }
variable "repo_root"             { type = string }
variable "apply_cluster_issuer"  {
  type = bool
  default = false
}
variable "letsencrypt_email" {
  type        = string
  description = "Email used for ACME registration with Let's Encrypt."
}
