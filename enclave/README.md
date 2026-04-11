# Aava Enclave on GCP Confidential Spaces

This directory contains the enclave service and GCP deployment stack for the Aava session engine.
It is adapted from `nautilus-gcp-confidential-spaces-poc` but targets this service's API and runtime needs.

## What this deploys

- Rust service (`session-enclave`) from this directory (`Cargo.toml`, `src/`)
- GCP Confidential Space VM (AMD SEV + secure boot)
- Container image in Artifact Registry
- Port `3000` ingress firewall rule
- Runtime env injection via `tee-env-*` metadata

Main files:

- `Containerfile.session_enclave`: container build for Confidential Spaces
- `deploy.sh`: build/push/apply helper
- `Makefile`: common commands
- `conf_space.tf`, `variables.tf`: infrastructure
- `terraform.tfvars.example`: local config template
- `gcp_registration_guide.md`: reference guide for on-chain registration flow

## API surface (service behavior)

- `GET /attestation`
- `GET /health_check` (minimal health endpoint)
- Internal backend-only routes:
  - `POST /internal/sessions/open`
  - `POST /internal/sessions/close`
  - `POST /internal/sessions/warn`
  - `POST /internal/sessions/revoke`
  - `POST /internal/sessions/get`
  - `POST /internal/streams/end`
  - `POST /internal/streams/cleanup`

Internal routes are protected by `X-Internal-Token` and must match `ENCLAVE_INTERNAL_TOKEN`.

Health response is intentionally minimal:

```json
{
  "status": "running",
  "pk": "<hex-encoded ephemeral public key>"
}
```

## Prerequisites

- `gcloud` CLI installed and authenticated
- Docker with BuildKit
- Terraform >= 1.3
- Existing Redis reachable from the Confidential Space VM (`REDIS_URL`)
- GCP project with APIs enabled:
  - `compute.googleapis.com`
  - `artifactregistry.googleapis.com`

## One-time GCP setup

```bash
gcloud config set project YOUR_PROJECT_ID
gcloud auth application-default login

gcloud services enable compute.googleapis.com
gcloud services enable artifactregistry.googleapis.com

gcloud iam service-accounts create confspace-runner \
  --display-name="Confidential Space Runner"

gcloud projects add-iam-policy-binding YOUR_PROJECT_ID \
  --member="serviceAccount:confspace-runner@YOUR_PROJECT_ID.iam.gserviceaccount.com" \
  --role="roles/confidentialcomputing.workloadUser"
```

## Configuration

Create your local tfvars file:

```bash
cp terraform.tfvars.example terraform.tfvars
```

Required values in `terraform.tfvars`:

- `project_id`
- `enclave_internal_token` (must match backend `ENCLAVE_INTERNAL_TOKEN`)
- `redis_url` (reachable from VM)

Optional values:

- `image_tag`, `region`, `zone`
- `allowed_source_ranges`, `create_firewall_rule`
- `rust_log`

## Build and deploy

```bash
cd enclave
make init
make deploy
```

Equivalent:

- `make build`: local image build only
- `make push`: build + push image, skip Terraform apply
- `make plan`: Terraform plan
- `make destroy`: tear down infra

After deploy:

```bash
IP=$(terraform output -raw vm_external_ip)
curl "http://$IP:3000/health_check"
curl "http://$IP:3000/attestation"
```

## Operations and maintenance

- **Rotate image**
  - Merge code changes
  - Run `make deploy` (uses git short SHA tag by default)
  - VM is replaced when image ref changes

- **Rotate internal token**
  - Update backend `ENCLAVE_INTERNAL_TOKEN`
  - Update `terraform.tfvars` `enclave_internal_token`
  - Re-run `make deploy`

- **Update Redis endpoint**
  - Change `redis_url` in `terraform.tfvars`
  - Re-run `make deploy`

- **Restrict ingress**
  - Set `allowed_source_ranges` in `terraform.tfvars`
  - Re-run `make deploy`

- **Destroy environment**
  - `make destroy`

## Troubleshooting

- **`terraform: command not found`**
  - Install Terraform and re-run `make init`

- **Service unreachable on port 3000**
  - Check VM external IP output
  - Verify firewall source ranges
  - Verify service is running in the VM logs

- **`/attestation` fails outside GCP Confidential Space**
  - Expected if TEE socket is missing
  - Test attestation on deployed Confidential Space VM

- **Internal routes return `401 Unauthorized`**
  - Backend `X-Internal-Token` does not match `ENCLAVE_INTERNAL_TOKEN`

- **Redis errors in enclave logs**
  - Validate `redis_url` connectivity from GCP VM
  - Verify Redis auth/network policy

## Notes

- Container env overrides are explicitly allowlisted in `Containerfile.session_enclave` via `tee.launch_policy.allow_env_override`.

