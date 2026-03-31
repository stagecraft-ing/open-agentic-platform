resource "helm_release" "ingress_nginx" {
  count      = var.ingress_nginx_enabled ? 1 : 0
  name       = "ingress-nginx"
  repository = "https://kubernetes.github.io/ingress-nginx"
  chart      = "ingress-nginx"
  namespace  = "ingress-nginx"
  create_namespace = true

  set {
    name  = "controller.service.annotations.service\\.beta\\.kubernetes\\.io/azure-load-balancer-health-probe-request-path"
    value = "/healthz"
  }
}

resource "helm_release" "cert_manager" {
  name             = "cert-manager"
  repository       = "https://charts.jetstack.io"
  chart            = "cert-manager"
  namespace        = "cert-manager"
  create_namespace = true
  version          = "v1.19.3"
  wait             = true

  set {
    name  = "installCRDs"
    value = "true"
  }
}

# csi-azure-provider chart includes secrets-store-csi-driver as a dependency - install both together
resource "helm_release" "csi_azure_provider" {
  count      = var.csi_secrets_enabled ? 1 : 0
  name       = "csi-azure-provider"
  repository = "https://azure.github.io/secrets-store-csi-driver-provider-azure/charts"
  chart      = "csi-secrets-store-provider-azure"
  namespace  = "kube-system"
}

resource "time_sleep" "wait_for_cert_manager_crds" {
  create_duration = "90s"
  depends_on      = [helm_release.cert_manager]
}

resource "kubernetes_manifest" "letsencrypt_cluster_issuer" {
  count = var.apply_cluster_issuer ? 1 : 0

  manifest = {
    apiVersion = "cert-manager.io/v1"
    kind       = "ClusterIssuer"
    metadata   = { name = "letsencrypt-prod" }
    spec = {
      acme = {
        server = "https://acme-v02.api.letsencrypt.org/directory"
        email  = var.letsencrypt_email
        privateKeySecretRef = { name = "letsencrypt-prod" }
        solvers = [{
          http01 = { ingress = { class = "nginx" } }
        }]
      }
    }
  }

  depends_on = [
    helm_release.cert_manager,
    time_sleep.wait_for_cert_manager_crds,
  ]
}
