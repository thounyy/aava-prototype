variable "project_id" {
  description = "GCP project ID"
  type        = string
}

variable "image_tag" {
  description = "Docker image tag to deploy"
  type        = string
  default     = "latest"
}

variable "region" {
  description = "GCP region"
  type        = string
  default     = "us-central1"
}

variable "zone" {
  description = "GCP zone"
  type        = string
  default     = "us-central1-b"
}

variable "allowed_source_ranges" {
  description = "Source IP ranges allowed to reach port 3000"
  type        = list(string)
  default     = ["0.0.0.0/0"]
}

variable "create_firewall_rule" {
  description = "Set to false if allow-cs-3000-ingress already exists in your project"
  type        = bool
  default     = true
}

variable "enclave_internal_token" {
  description = "Shared auth token used by backend -> enclave internal endpoints"
  type        = string
  sensitive   = true
}

variable "redis_url" {
  description = "Redis URL reachable from Confidential Space VM"
  type        = string
}

variable "rust_log" {
  description = "RUST_LOG value injected into enclave server"
  type        = string
  default     = "info"
}

