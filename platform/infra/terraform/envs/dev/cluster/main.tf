locals {
  core = data.terraform_remote_state.core.outputs
}

module "cluster_addons" {
  source = "../../../modules/cluster_addons"

  repo_root = "${path.module}/../../../../.."

  cloud_provider           = "azure"
  ingress_nginx_enabled    = true
  cert_manager_enabled     = true
  external_secrets_enabled = true

  apply_cluster_issuer = var.apply_cluster_issuer
  letsencrypt_email    = var.letsencrypt_email
}

module "external_secrets_config" {
  source = "../../../modules/external_secrets"

  cloud_provider    = "azure"
  secret_store_name = local.core.keyvault_name

  depends_on = [module.cluster_addons]
}

# Rauthy (OIDC provider) is deployed via platform/charts/rauthy Helm chart.
# It replaces the former logto_bootstrap module and uses hiqlite (embedded
# Raft SQLite) instead of a PostgreSQL database.
# TODO: Add rauthy Terraform module when moving beyond Helm-only deployment.

module "platform_bootstrap" {
  source = "../../../modules/platform_bootstrap"

  cloud_provider = "azure"
  registry_url   = local.core.acr_login_server

  stagecraft_namespace = "stagecraft-system"
  deployd_namespace    = "deployd-system"

  stagecraft_host = var.stagecraft_host
  deployd_host    = var.deployd_host

  stagecraft_sa_name = local.core.stagecraft_serviceaccount_name
  deployd_sa_name    = local.core.deployd_serviceaccount_name
  cloud_identity_id  = local.core.stagecraft_identity_client_id

  charts_root = "${path.module}/../../../../../charts"

  depends_on = [module.cluster_addons, module.external_secrets_config]
}
