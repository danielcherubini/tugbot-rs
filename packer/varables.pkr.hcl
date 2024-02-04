variable "discord_application_id" {
  type    = string
  default = env("DISCORD_APPLICATION_ID")
}

variable "discord_token" {
  type    = string
  default = env("DISCORD_TOKEN")
}

variable "database_url" {
  type    = string
  default = env("DATABASE_URL")
}

variable "proxmox_token" {
  type    = string
  default = env("PROXMOX_TOKEN")
}

variable "proxmox_url" {
  type    = string
  default = env("PROXMOX_URL")
}

variable "proxmox_username" {
  type    = string
  default = env("PROXMOX_USERNAME")
}

variable "git_version" {
  type    = string
  default = env("CI_COMMIT_TAG")
}

locals {
  template_name = "tugbot-template-${var.git_version}"
}
