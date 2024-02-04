packer {
  required_plugins {
    proxmox = {
      version = ">= 1.1.0"
      source  = "github.com/hashicorp/proxmox"
    }
  }
}

variable "discord_application_id" {
  type    = string
  default = "${env("DISCORD_APPLICATION_ID")}"
}

variable "discord_token" {
  type    = string
  default = "${env("DISCORD_TOKEN")}"
}

variable "database_url" {
  type    = string
  default = "${env("DATABASE_URL")}"
}

variable "proxmox_token" {
  type    = string
  default = "${env("PROXMOX_TOKEN")}"
}

variable "proxmox_url" {
  type    = string
  default = "${env("PROXMOX_URL")}"
}

variable "proxmox_username" {
  type    = string
  default = "${env("PROXMOX_USERNAME")}"
}

locals {
  template_name = "tugbot-template-${env("CI_COMMIT_TAG")}"
}

source "proxmox-clone" "tugbot" {
  clone_vm                 = "debian-11"
  cores                    = 4
  insecure_skip_tls_verify = true
  memory                   = 4096
  network_adapters {
    bridge = "vmbr1"
  }
  node                 = "jove"
  onboot               = true
  os                   = "l26"
  proxmox_url          = "${var.proxmox_url}"
  qemu_agent           = true
  sockets              = 1
  ssh_username         = "root"
  template_description = "image made from cloud-init image"
  template_name        = "${local.template_name}"
  token                = "${var.proxmox_token}"
  username             = "${var.proxmox_username}"
}

build {
  description = "Tugbot template build"

  sources = ["source.proxmox-clone.tugbot"]

  provisioner "shell" {
    pause_before = "30s"
    max_retries = 5
    inline = [
      "sleep 30",
      "sudo apt-get -y install git build-essential libpq-dev",
      "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- --profile minimal -y",
      ". $HOME/.cargo/env",
      "mkdir -p /usr/src/tugbot",
      "git clone https://gitlab.com/danielcherubini/tugbot-rs.git /usr/src/tugbot",
      "cd /usr/src/tugbot",
      "cargo install --path .",
      "echo 'DISCORD_TOKEN=${var.discord_token}\nAPPLICATION_ID=${var.discord_application_id}\nDATABASE_URL=${var.database_url}' > .env",
      "cp systemd/tugbot.service /etc/systemd/system",
      "systemctl daemon-reload",
      "systemctl enable tugbot.service"
    ]
  }
}
