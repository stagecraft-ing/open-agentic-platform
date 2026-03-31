resource "kubernetes_namespace" "logto" {
  metadata { name = var.namespace }
}

locals {
  scripts_concat = join("\n---\n", [
    file("${path.module}/../../../../scripts/logto/entrypoint.sh"),
    file("${path.module}/../../../../scripts/logto/setup.js"),
    file("${path.module}/../../../../scripts/logto/config.js"),
    file("${path.module}/../../../../scripts/logto/index.js")
  ])
  scripts_hash = sha256(local.scripts_concat)
}

resource "kubernetes_config_map" "logto_custom_setup" {
  metadata {
    name      = "logto-custom-setup"
    namespace = var.namespace
  }

  data = {
    "entrypoint.sh" = file("${path.module}/../../../../scripts/logto/entrypoint.sh")
    "setup.js"      = file("${path.module}/../../../../scripts/logto/setup.js")
    "config.js"     = file("${path.module}/../../../../scripts/logto/config.js")
    "index.js"      = file("${path.module}/../../../../scripts/logto/index.js")
  }
}

resource "kubernetes_secret" "logto_custom_setup_secrets" {
  metadata {
    name      = "logto-custom-setup-secrets"
    namespace = var.namespace
  }

  data = {
    "LOGTO_SPA_CLIENT_SECRET"                             = var.logto_spa_client_secret
    "LOGTO_M2M_CLIENT_SECRET"      = var.logto_m2m_client_secret
    "LOGTO_SPA_API_EVENT_WEBHOOK_SIGNING_KEY"      = var.logto_spa_api_event_webhook_signing_key
    "LOGTO_GOOGLE_CLIENT_SECRET"                   = var.logto_google_client_secret
    "LOGTO_GOOGLE_WORKSPACE_CLIENT_SECRET"         = var.logto_google_workspace_client_secret
  }
}

resource "helm_release" "logto" {
  name             = "logto"
  chart            = abspath("${var.charts_root}/logto")
  namespace        = var.namespace
  create_namespace = false

  values = [yamlencode({
    customSetup = {
      enabled       = true
      configMapName = kubernetes_config_map.logto_custom_setup.metadata[0].name
      configMapHash = local.scripts_hash
      failReleaseOnError = true
      env = {
        DOMAIN                                              = var.domain
        APP_NAME                                            = var.app_name
        APP_URL                                             = var.app_url
        LOGTO_SPA_CLIENT_ID                                        = var.logto_spa_client_id
        LOGTO_SPA_API_RESOURCE                              = var.logto_spa_api_resource
        LOGTO_M2M_CLIENT_ID                 = var.logto_m2m_client_id
        LOGTO_SPA_API_EVENT_WEBHOOK_URL                     = var.logto_spa_api_event_webhook_url
        LOGTO_GOOGLE_CONNECTOR_ID                           = var.logto_google_connector_id
        LOGTO_GOOGLE_CLIENT_ID                              = var.logto_google_client_id
        LOGTO_GOOGLE_WORKSPACE_CONNECTOR_ID                 = var.logto_google_workspace_connector_id
        LOGTO_GOOGLE_WORKSPACE_CLIENT_ID                    = var.logto_google_workspace_client_id
        LOGTO_GOOGLE_WORKSPACE_CONNECTOR_APPROVED_DOMAINS   = var.logto_google_workspace_connector_approved_domains
      }
    }

    customSetupSecrets = {
      enabled    = true
      secretName = kubernetes_secret.logto_custom_setup_secrets.metadata[0].name
    }

    logto = {
      endpoint         = "https://${var.logto_host}"
      adminEndpoint    = "https://${var.admin_host}"
      trustProxyHeader = true
    }

    ingress = {
      enabled   = true
      className = "nginx"
      annotations = {
        "cert-manager.io/cluster-issuer" = "letsencrypt-prod"
        "nginx.ingress.kubernetes.io/proxy-body-size" = "10m"
      }
      tls = [
        { secretName = "logto-tls", hosts = [var.logto_host] },
        { secretName = "logto-admin-tls", hosts = [var.admin_host] }
      ]
      hosts = [
        { host = var.logto_host, paths = [{ path = "/", pathType = "Prefix" }] },
        { host = var.admin_host, paths = [{ path = "/", pathType = "Prefix" }] }
      ]
    }

    postgresql = {
      enabled = true
      auth = {
        username = "postgres"
        password = var.postgres_password
        database = "logto"
      }
      persistence = { enabled = true, size = "20Gi" }
    }

    image = {
      repository = "svhd/logto"
      tag        = "1.36.0"
    }
  })]

  timeout = 900
  wait    = true
}
