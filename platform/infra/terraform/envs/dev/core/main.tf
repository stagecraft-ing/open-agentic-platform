module "azure_core" {
  source       = "../../../modules/azure_core"
  project_name = var.project_name
  location     = var.location
}

module "workload_identity" {
  source              = "../../../modules/workload_identity"
  resource_group_name = module.azure_core.resource_group_name
  location            = var.location
  aks_name            = module.azure_core.aks_name
  aks_oidc_issuer_url = module.azure_core.aks_oidc_issuer_url
  keyvault_id         = module.azure_core.keyvault_id
}

# Wait for Key Vault RBAC role assignment to propagate before creating secrets
resource "time_sleep" "wait_for_kv_rbac" {
  create_duration = "90s"
  depends_on      = [module.azure_core]
}

module "keyvault_secrets" {
  source        = "../../../modules/keyvault_secrets"
  keyvault_id   = module.azure_core.keyvault_id
  keyvault_name = module.azure_core.keyvault_name

  secrets = {
    OIDC_M2M_CLIENT_ID     = var.oidc_m2m_client_id
    OIDC_M2M_CLIENT_SECRET = var.oidc_m2m_client_secret
    STAGECRAFT_DB_URL      = var.stagecraft_db_url
    DEPLOYD_DB_URL         = var.deployd_db_url
  }

  depends_on = [time_sleep.wait_for_kv_rbac]
}
