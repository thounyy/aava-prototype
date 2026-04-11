terraform {
  required_version = ">= 1.3"
  required_providers {
    google = {
      source  = "hashicorp/google"
      version = "~> 5.0"
    }
    random = {
      source  = "hashicorp/random"
      version = "~> 3.6"
    }
    null = {
      source  = "hashicorp/null"
      version = "~> 3.0"
    }
  }
}

resource "random_pet" "vm_suffix" {
  length = 2
}

locals {
  repo       = "confspace-aava"
  image_name = "session-enclave"

  vm_base = "confspace-enclave"
  vm_name = "${local.vm_base}-${random_pet.vm_suffix.id}"

  full_ref = "${var.region}-docker.pkg.dev/${var.project_id}/${local.repo}/${local.image_name}:${var.image_tag}"
}

provider "google" {
  project = var.project_id
  region  = var.region
  zone    = var.zone
}

resource "null_resource" "image_ref" {
  triggers = {
    image_ref = local.full_ref
  }
}

resource "google_artifact_registry_repository" "confspace" {
  location      = var.region
  repository_id = local.repo
  description   = "Docker images for Aava Confidential Spaces enclave"
  format        = "DOCKER"
}

resource "google_compute_firewall" "cs_3000" {
  count = var.create_firewall_rule ? 1 : 0

  name          = "allow-cs-3000-ingress"
  network       = "default"
  direction     = "INGRESS"
  priority      = 1000
  source_ranges = var.allowed_source_ranges
  target_tags   = ["cs-3000"]

  allow {
    protocol = "tcp"
    ports    = ["3000"]
  }
}

resource "google_compute_instance" "confspace_vm" {
  name         = local.vm_name
  zone         = var.zone
  machine_type = "n2d-standard-2"
  tags         = ["cs-3000"]

  labels = {
    app     = "aava-enclave"
    managed = "terraform"
  }

  confidential_instance_config {
    enable_confidential_compute = true
  }

  shielded_instance_config {
    enable_secure_boot = true
  }

  boot_disk {
    initialize_params {
      image = "projects/confidential-space-images/global/images/family/confidential-space"
      size  = 50
    }
  }

  network_interface {
    network = "default"
    access_config {}
  }

  metadata = {
    tee-image-reference           = local.full_ref
    tee-env-ENCLAVE_INTERNAL_TOKEN = var.enclave_internal_token
    tee-env-REDIS_URL             = var.redis_url
    tee-env-RUST_LOG              = var.rust_log
  }

  service_account {
    email  = "confspace-runner@${var.project_id}.iam.gserviceaccount.com"
    scopes = ["https://www.googleapis.com/auth/cloud-platform"]
  }

  lifecycle {
    replace_triggered_by = [null_resource.image_ref]
  }

  depends_on = [google_compute_firewall.cs_3000]
}

output "vm_external_ip" {
  value = google_compute_instance.confspace_vm.network_interface[0].access_config[0].nat_ip
}

output "curl_health_check" {
  value = "curl http://${google_compute_instance.confspace_vm.network_interface[0].access_config[0].nat_ip}:3000/health_check"
}

output "curl_attestation" {
  value = "curl http://${google_compute_instance.confspace_vm.network_interface[0].access_config[0].nat_ip}:3000/attestation"
}

