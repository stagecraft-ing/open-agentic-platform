output "kube_host" {
  value = module.eks.cluster_endpoint
}

output "kube_client_certificate" {
  value       = ""
  description = "EKS uses token-based auth, not client certificates"
}

output "kube_client_key" {
  value       = ""
  description = "EKS uses token-based auth, not client certificates"
}

output "kube_cluster_ca_certificate" {
  value = module.eks.cluster_certificate_authority_data
}

output "cluster_name" {
  value = module.eks.cluster_name
}

output "oidc_issuer_url" {
  value = module.eks.cluster_oidc_issuer_url
}

output "registry_url" {
  value       = split("/", aws_ecr_repository.stagecraft.repository_url)[0]
  description = "ECR registry URL (without repository name)"
}

output "resource_group_or_project" {
  value = data.aws_caller_identity.current.account_id
}

output "secret_store_name" {
  value       = "aws-secrets-manager"
  description = "Logical name for the secret store — used in ESO ClusterSecretStore"
}

output "stagecraft_identity_id" {
  value = module.stagecraft_irsa.iam_role_arn
}

output "deployd_identity_id" {
  value = module.deployd_irsa.iam_role_arn
}

output "eso_role_arn" {
  value = module.eso_irsa.iam_role_arn
}

output "stagecraft_serviceaccount_name" { value = "stagecraft-api-sa" }
output "deployd_serviceaccount_name"    { value = "deployd-api-sa" }
