REGISTRY := local
TAG      := $(shell git rev-parse --short HEAD 2>/dev/null || echo "latest")

.DEFAULT_GOAL := build
.PHONY: build deploy push init plan destroy

build:
	docker build \
		--tag $(REGISTRY)/session-enclave \
		--progress=plain \
		--platform linux/amd64 \
		--provenance=false \
		-f Containerfile.session_enclave \
		.

init:
	terraform init

plan:
	terraform plan -var "project_id=$$(gcloud config get-value project 2>/dev/null)"

deploy:
	IMAGE_TAG=$(TAG) bash deploy.sh

push:
	IMAGE_TAG=$(TAG) SKIP_TERRAFORM=1 bash deploy.sh

destroy:
	terraform destroy -var "project_id=$$(gcloud config get-value project 2>/dev/null)"

