resource "kubernetes_namespace" "stagecraft" {
  metadata { name = var.stagecraft_namespace }
}

resource "kubernetes_namespace" "deployd" {
  metadata { name = var.deployd_namespace }
}

resource "helm_release" "stagecraft" {
  name      = "stagecraft"
  chart     = abspath("${var.charts_root}/stagecraft")
  namespace = var.stagecraft_namespace
  create_namespace = false

  values = [yamlencode({
    image = { repository = "stagecraftdevacr.azurecr.io/stagecraft", tag = "dev" }

    ingress = { host = var.stagecraft_host }

    workloadIdentity = {
      serviceAccountName = var.stagecraft_sa_name
      clientId           = var.stagecraft_client_id
    }

    keyVault = {
      name     = var.keyvault_name
      tenantId = var.tenant_id
    }

    secretsMount = {
      enabled = true
      mountPath = "/mnt/secrets-store"
      objects = [
        { objectName = "stagecraft-db-url", objectAlias = "STAGECRAFT_DB_URL", objectType = "secret" },
        { objectName = "logto-m2m-client-id", objectAlias = "LOGTO_M2M_CLIENT_ID", objectType = "secret" },
        { objectName = "logto-m2m-client-secret", objectAlias = "LOGTO_M2M_CLIENT_SECRET", objectType = "secret" }
      ]
    }
  })]

  timeout = 900
  wait    = true
}

resource "helm_release" "deployd_api" {
  name      = "deployd-api"
  chart     = abspath("${var.charts_root}/deployd-api")
  namespace = var.deployd_namespace
  create_namespace = false

  values = [yamlencode({
    image = { repository = "stagecraftdevacr.azurecr.io/deployd-api", tag = "dev" }

    ingress = { host = var.deployd_host }

    workloadIdentity = {
      serviceAccountName = var.deployd_sa_name
      clientId           = var.deployd_client_id
    }

    keyVault = {
      name     = var.keyvault_name
      tenantId = var.tenant_id
    }

    secretsMount = {
      enabled = true
      mountPath = "/mnt/secrets-store"
      objects = [
        { objectName = "deployd-db-url", objectAlias = "DEPLOYD_DB_URL", objectType = "secret" }
      ]
    }
  })]

  timeout = 900
  wait    = true
}
