locals {
  core = data.terraform_remote_state.core.outputs
}

module "cluster_addons" {
  source = "../../../modules/cluster_addons"

  repo_root = "${path.module}/../../../../.."

  ingress_nginx_enabled = true
  cert_manager_enabled  = true
  csi_secrets_enabled   = true

  apply_cluster_issuer = var.apply_cluster_issuer
  letsencrypt_email    = var.letsencrypt_email
}

module "logto_bootstrap" {
  source      = "../../../modules/logto_bootstrap"
  charts_root = "${path.module}/../../../../../charts"
  namespace   = "logto"

  logto_host = var.logto_host
  admin_host = var.logto_admin_host

  postgres_password = var.logto_postgres_password

  domain                                            = var.domain
  logto_spa_client_id                               = var.logto_spa_client_id
  logto_spa_api_resource                            = var.logto_spa_api_resource
  logto_m2m_client_id                               = var.logto_m2m_client_id
  app_name                                          = var.app_name
  app_url                                           = var.app_url
  logto_spa_client_secret                           = var.logto_spa_client_secret
  logto_spa_api_event_webhook_url                   = var.logto_spa_api_event_webhook_url
  logto_m2m_client_secret                           = var.logto_m2m_client_secret
  logto_spa_api_event_webhook_signing_key           = var.logto_spa_api_event_webhook_signing_key
  logto_google_connector_id                         = var.logto_google_connector_id
  logto_google_client_id                            = var.logto_google_client_id
  logto_google_client_secret                        = var.logto_google_client_secret
  logto_google_workspace_connector_id               = var.logto_google_workspace_connector_id
  logto_google_workspace_client_id                  = var.logto_google_workspace_client_id
  logto_google_workspace_client_secret              = var.logto_google_workspace_client_secret
  logto_google_workspace_connector_approved_domains = var.logto_google_workspace_connector_approved_domains

  depends_on = [module.cluster_addons]
}

module "platform_bootstrap" {
  source = "../../../modules/platform_bootstrap"

  stagecraft_namespace = "stagecraft-system"
  deployd_namespace    = "deployd-system"

  stagecraft_host = var.stagecraft_host
  deployd_host    = var.deployd_host

  stagecraft_sa_name   = local.core.stagecraft_serviceaccount_name
  deployd_sa_name      = local.core.deployd_serviceaccount_name
  stagecraft_client_id = local.core.stagecraft_identity_client_id
  deployd_client_id    = local.core.deployd_identity_client_id

  keyvault_name = local.core.keyvault_name
  tenant_id     = local.core.tenant_id

  charts_root = "${path.module}/../../../../../charts"

  depends_on = [module.cluster_addons]
}
