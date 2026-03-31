# Stagecraft Platform (Terraform-first)

## Prereqs
- Azure CLI logged in
- Terraform
- kubectl
- Helm

## Bring up dev
cd infra/terraform/envs/dev
cp terraform.tfvars.example terraform.tfvars
terraform init
terraform apply -target=module.azure_core
terraform apply -target=module.cluster_addons.helm_release.cert_manager
terraform apply

## Get kubeconfig
az aks get-credentials -g <rg> -n <aks_name> --overwrite-existing
kubectl get ns

az acr login -n stagecraftdevacr
cd services/deployd-api && docker build -t stagecraftdevacr.azurecr.io/deployd-api:dev . && docker push stagecraftdevacr.azurecr.io/deployd-api:dev
cd services/stagecraft && encore build docker --config ./infra.config.json stagecraftdevacr.azurecr.io/stagecraft:dev && docker push stagecraftdevacr.azurecr.io/stagecraft:dev
