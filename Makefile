ENV ?= dev
TF_CORE   = infra/terraform/envs/$(ENV)/core
TF_CLUSTER = infra/terraform/envs/$(ENV)/cluster

tf-init:
	cd $(TF_CORE) && terraform init
	cd $(TF_CLUSTER) && terraform init

docker-push:
	az acr login -n stagecraftdevacr
	cd services/deployd-api && docker build -t stagecraftdevacr.azurecr.io/deployd-api:dev . && docker push stagecraftdevacr.azurecr.io/deployd-api:dev
	cd services/stagecraft && encore build docker --config ./infra.config.json stagecraftdevacr.azurecr.io/stagecraft:dev && docker push stagecraftdevacr.azurecr.io/stagecraft:dev

tf-apply:
	cd $(TF_CORE) && terraform apply -auto-approve
	$(MAKE) docker-push
	cd $(TF_CLUSTER) && terraform apply -auto-approve \
		-target=module.cluster_addons.helm_release.cert_manager \
		-target=module.cluster_addons.helm_release.ingress_nginx \
		-target=module.cluster_addons.helm_release.csi_azure_provider \
		-target=module.cluster_addons.time_sleep.wait_for_cert_manager_crds
	cd $(TF_CLUSTER) && terraform apply -auto-approve

az-cleanup:
	az network watcher configure --enabled false --locations canadacentral
	az group delete --name NetworkWatcherRG --yes --no-wait
	az group delete --name stagecraft-dev-rg --yes --no-wait

tf-destroy:
	cd $(TF_CLUSTER) && terraform destroy -auto-approve
	cd $(TF_CORE) && terraform destroy -auto-approve
	$(MAKE) az-cleanup
