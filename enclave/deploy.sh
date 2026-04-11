#!/usr/bin/env bash
set -euo pipefail

PROJECT_ID=$(gcloud config get-value project 2>/dev/null)
if [[ -z "${PROJECT_ID}" ]]; then
  echo "ERROR: No GCP project configured. Run: gcloud config set project YOUR_PROJECT_ID" >&2
  exit 1
fi

REGION="${REGION:-us-central1}"
REPO="${REPO:-confspace-aava}"
IMAGE_NAME="${IMAGE_NAME:-session-enclave}"
TAG="${IMAGE_TAG:-$(git rev-parse --short HEAD)}"

FULL_REF="${REGION}-docker.pkg.dev/${PROJECT_ID}/${REPO}/${IMAGE_NAME}:${TAG}"

echo "==> Configuring Docker auth for Artifact Registry..."
gcloud auth configure-docker "${REGION}-docker.pkg.dev" --quiet

echo "==> Building image: ${FULL_REF}"
docker build \
  --platform linux/amd64 \
  --provenance=false \
  -t "${FULL_REF}" \
  -f Containerfile.session_enclave \
  .

echo "==> Pushing image: ${FULL_REF}"
docker push "${FULL_REF}"

if [[ "${SKIP_TERRAFORM:-0}" == "1" ]]; then
  echo "==> SKIP_TERRAFORM=1 set; skipping terraform apply."
  exit 0
fi

echo "==> Running terraform apply (project=${PROJECT_ID}, tag=${TAG})..."
terraform apply \
  -var "project_id=${PROJECT_ID}" \
  -var "image_tag=${TAG}"

