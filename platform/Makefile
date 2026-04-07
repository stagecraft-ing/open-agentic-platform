# OAP Platform — Multi-Target Deployment
# Usage: make deploy TARGET=azure|aws|gcp|do|hetzner|local

TARGET ?= azure
ENV    ?= dev

# --- Directory layout ---
TF_ROOT      = infra/terraform
TF_MODULES   = $(TF_ROOT)/modules
CHARTS_ROOT  = charts
HETZNER_ROOT = infra/hetzner
LOCAL_ROOT   = infra/local

# --- Terraform env paths (cloud targets) ---
TF_CORE    = $(TF_ROOT)/envs/$(ENV)/core
TF_CLUSTER = $(TF_ROOT)/envs/$(ENV)/cluster

# ============================================================
# Top-level targets
# ============================================================

.PHONY: deploy destroy bootstrap info

deploy:
	@$(MAKE) deploy-$(TARGET)

destroy:
	@$(MAKE) destroy-$(TARGET)

bootstrap:
	@$(MAKE) bootstrap-$(TARGET)

info:
	@echo "TARGET=$(TARGET)  ENV=$(ENV)"
	@echo "TF_CORE=$(TF_CORE)"
	@echo "TF_CLUSTER=$(TF_CLUSTER)"

# ============================================================
# Azure
# ============================================================

.PHONY: deploy-azure destroy-azure

tf-init:
	cd $(TF_CORE) && terraform init
	cd $(TF_CLUSTER) && terraform init

docker-push-azure:
	az acr login -n stagecraftdevacr
	cd services/deployd-api-rs && docker build -t stagecraftdevacr.azurecr.io/deployd-api:dev . && docker push stagecraftdevacr.azurecr.io/deployd-api:dev
	cd services/stagecraft && encore build docker --config ./infra.config.json stagecraftdevacr.azurecr.io/stagecraft:dev && docker push stagecraftdevacr.azurecr.io/stagecraft:dev

deploy-azure: tf-init
	cd $(TF_CORE) && terraform apply -auto-approve
	$(MAKE) docker-push-azure
	cd $(TF_CLUSTER) && terraform apply -auto-approve \
		-target=module.cluster_addons.helm_release.cert_manager \
		-target=module.cluster_addons.helm_release.ingress_nginx \
		-target=module.cluster_addons.helm_release.external_secrets \
		-target=module.cluster_addons.time_sleep.wait_for_cert_manager_crds
	cd $(TF_CLUSTER) && terraform apply -auto-approve

destroy-azure:
	cd $(TF_CLUSTER) && terraform destroy -auto-approve
	cd $(TF_CORE) && terraform destroy -auto-approve
	az network watcher configure --enabled false --locations canadacentral
	az group delete --name NetworkWatcherRG --yes --no-wait
	az group delete --name stagecraft-dev-rg --yes --no-wait

# ============================================================
# AWS
# ============================================================

.PHONY: deploy-aws destroy-aws

deploy-aws:
	cd $(TF_ROOT)/envs/aws-$(ENV)/core && terraform init && terraform apply -auto-approve
	cd $(TF_ROOT)/envs/aws-$(ENV)/cluster && terraform init && terraform apply -auto-approve

destroy-aws:
	cd $(TF_ROOT)/envs/aws-$(ENV)/cluster && terraform destroy -auto-approve
	cd $(TF_ROOT)/envs/aws-$(ENV)/core && terraform destroy -auto-approve

# ============================================================
# GCP
# ============================================================

.PHONY: deploy-gcp destroy-gcp

deploy-gcp:
	cd $(TF_ROOT)/envs/gcp-$(ENV)/core && terraform init && terraform apply -auto-approve
	cd $(TF_ROOT)/envs/gcp-$(ENV)/cluster && terraform init && terraform apply -auto-approve

destroy-gcp:
	cd $(TF_ROOT)/envs/gcp-$(ENV)/cluster && terraform destroy -auto-approve
	cd $(TF_ROOT)/envs/gcp-$(ENV)/core && terraform destroy -auto-approve

# ============================================================
# DigitalOcean
# ============================================================

.PHONY: deploy-do destroy-do

deploy-do:
	cd $(TF_ROOT)/envs/do-$(ENV)/core && terraform init && terraform apply -auto-approve
	cd $(TF_ROOT)/envs/do-$(ENV)/cluster && terraform init && terraform apply -auto-approve

destroy-do:
	cd $(TF_ROOT)/envs/do-$(ENV)/cluster && terraform destroy -auto-approve
	cd $(TF_ROOT)/envs/do-$(ENV)/core && terraform destroy -auto-approve

# ============================================================
# Hetzner K3s
# ============================================================

.PHONY: deploy-hetzner destroy-hetzner bootstrap-hetzner

bootstrap-hetzner:
	@echo "==> Running Hetzner setup..."
	$(HETZNER_ROOT)/setup.sh

deploy-hetzner:
	KUBECONFIG=$(HETZNER_ROOT)/kubeconfig helm upgrade --install rauthy $(CHARTS_ROOT)/rauthy \
		--namespace rauthy-system \
		-f $(CHARTS_ROOT)/rauthy/values.yaml \
		-f $(CHARTS_ROOT)/rauthy/values-hetzner.yaml \
		--wait --timeout 300s
	KUBECONFIG=$(HETZNER_ROOT)/kubeconfig helm upgrade --install stagecraft $(CHARTS_ROOT)/stagecraft \
		--namespace stagecraft-system \
		-f $(CHARTS_ROOT)/stagecraft/values.yaml \
		-f $(CHARTS_ROOT)/stagecraft/values-hetzner.yaml \
		--wait --timeout 300s
	KUBECONFIG=$(HETZNER_ROOT)/kubeconfig helm upgrade --install deployd-api $(CHARTS_ROOT)/deployd-api \
		--namespace deployd-system \
		-f $(CHARTS_ROOT)/deployd-api/values.yaml \
		-f $(CHARTS_ROOT)/deployd-api/values-hetzner.yaml \
		--wait --timeout 300s

docker-build-hetzner:
	cd services/deployd-api-rs && docker build -t ghcr.io/open-agentic-platform/deployd-api:latest .
	cd services/stagecraft && encore build docker --config ./infra.config.hetzner.json ghcr.io/open-agentic-platform/stagecraft:latest

docker-push-hetzner: docker-build-hetzner
	docker push ghcr.io/open-agentic-platform/deployd-api:latest
	docker push ghcr.io/open-agentic-platform/stagecraft:latest

destroy-hetzner:
	hetzner-k3s delete --config $(HETZNER_ROOT)/cluster.yaml

# ============================================================
# Local (k3d / kind)
# ============================================================

.PHONY: deploy-local destroy-local bootstrap-local

LOCAL_RUNTIME ?= k3d

bootstrap-local:
	$(LOCAL_ROOT)/bootstrap.sh $(LOCAL_RUNTIME)

deploy-local:
	helm upgrade --install stagecraft $(CHARTS_ROOT)/stagecraft \
		--namespace stagecraft-system \
		-f $(CHARTS_ROOT)/stagecraft/values.yaml \
		-f $(CHARTS_ROOT)/stagecraft/values-local.yaml \
		--wait --timeout 300s
	helm upgrade --install deployd-api $(CHARTS_ROOT)/deployd-api \
		--namespace deployd-system \
		-f $(CHARTS_ROOT)/deployd-api/values.yaml \
		-f $(CHARTS_ROOT)/deployd-api/values-local.yaml \
		--wait --timeout 300s

destroy-local:
ifeq ($(LOCAL_RUNTIME),k3d)
	k3d cluster delete oap-local
else
	kind delete cluster --name oap-local
endif

# ============================================================
# Docker build (generic — push to any registry)
# ============================================================

.PHONY: docker-build docker-push

REGISTRY ?= ghcr.io/open-agentic-platform

docker-build:
	cd services/deployd-api && docker build -t $(REGISTRY)/deployd-api:dev .
	cd services/stagecraft && encore build docker --config ./infra.config.json $(REGISTRY)/stagecraft:dev

docker-push: docker-build
	docker push $(REGISTRY)/deployd-api:dev
	docker push $(REGISTRY)/stagecraft:dev
