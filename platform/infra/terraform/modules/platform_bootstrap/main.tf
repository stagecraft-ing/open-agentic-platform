resource "kubernetes_namespace" "stagecraft" {
  metadata { name = var.stagecraft_namespace }
}

resource "kubernetes_namespace" "deployd" {
  metadata { name = var.deployd_namespace }
}

locals {
  secrets_provider = var.cloud_provider == "local" ? "k8s" : "eso"

  # Per-cloud service account annotations
  sa_annotations = {
    azure   = var.cloud_identity_id != "" ? { "azure.workload.identity/client-id" = var.cloud_identity_id } : {}
    aws     = var.cloud_identity_id != "" ? { "eks.amazonaws.com/role-arn" = var.cloud_identity_id } : {}
    gcp     = var.cloud_identity_id != "" ? { "iam.gke.io/gcp-service-account" = var.cloud_identity_id } : {}
    do      = {}
    hetzner = {}
    local   = {}
  }

  # Per-cloud pod labels
  pod_labels = {
    azure   = { "azure.workload.identity/use" = "true" }
    aws     = {}
    gcp     = {}
    do      = {}
    hetzner = {}
    local   = {}
  }

  # ESO secret store ref name per cloud
  secret_store_name = {
    azure   = "azure-keyvault"
    aws     = "aws-secrets-manager"
    gcp     = "gcp-secret-manager"
    do      = "do-vault"
    hetzner = ""
    local   = ""
  }
}

resource "helm_release" "stagecraft" {
  name             = "stagecraft"
  chart            = abspath("${var.charts_root}/stagecraft")
  namespace        = var.stagecraft_namespace
  create_namespace = false

  values = [yamlencode({
    image = {
      repository = var.registry_url != "" ? "${var.registry_url}/stagecraft" : "ghcr.io/open-agentic-platform/stagecraft"
      tag        = "dev"
    }
    ingress = { host = var.stagecraft_host }
    serviceAccount = {
      name        = var.stagecraft_sa_name
      annotations = lookup(local.sa_annotations, var.cloud_provider, {})
    }
    podLabels = lookup(local.pod_labels, var.cloud_provider, {})
    secrets = {
      enabled   = true
      provider  = local.secrets_provider
      mountPath = "/mnt/secrets-store"
      storeRef = {
        name = lookup(local.secret_store_name, var.cloud_provider, "")
        kind = "ClusterSecretStore"
      }
      keys = [
        { key = "STAGECRAFT_DB_URL", remoteKey = "stagecraft-db-url" },
        { key = "LOGTO_M2M_CLIENT_ID", remoteKey = "logto-m2m-client-id" },
        { key = "LOGTO_M2M_CLIENT_SECRET", remoteKey = "logto-m2m-client-secret" }
      ]
    }
  })]

  timeout = 900
  wait    = true
}

resource "helm_release" "deployd_api" {
  name             = "deployd-api"
  chart            = abspath("${var.charts_root}/deployd-api")
  namespace        = var.deployd_namespace
  create_namespace = false

  values = [yamlencode({
    image = {
      repository = var.registry_url != "" ? "${var.registry_url}/deployd-api" : "ghcr.io/open-agentic-platform/deployd-api"
      tag        = "dev"
    }
    ingress = { host = var.deployd_host }
    serviceAccount = {
      name        = var.deployd_sa_name
      annotations = lookup(local.sa_annotations, var.cloud_provider, {})
    }
    podLabels = lookup(local.pod_labels, var.cloud_provider, {})
    secrets = {
      enabled   = true
      provider  = local.secrets_provider
      mountPath = "/mnt/secrets-store"
      storeRef = {
        name = lookup(local.secret_store_name, var.cloud_provider, "")
        kind = "ClusterSecretStore"
      }
      keys = [
        { key = "DEPLOYD_DB_URL", remoteKey = "deployd-db-url" }
      ]
    }
  })]

  timeout = 900
  wait    = true
}
