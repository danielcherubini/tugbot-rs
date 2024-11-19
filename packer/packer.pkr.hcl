packer {
  required_plugins {
    proxmox = {
      version = ">= 1.1.0"
      source  = "github.com/hashicorp/proxmox"
    }
  }
}

source "proxmox-iso" "tugbot" {
  iso_storage             = "local"
  iso_url                 = "http://cdimage.debian.org/debian-cd/current/amd64/iso-cd/debian-12.8.0-amd64-netinst.iso"
  iso_checksum            = "sha256:04396d12b0f377958a070c38a923c227832fa3b3e18ddc013936ecf492e9fbb3"
  iso_checksum_type       = "sha256"
  vm_name                 = "tugbot"
  node                    = "jove"
  storage_pool            = "local-lvm"
  cores                   = 4
  memory                  = 4096
  ssh_username            = "root"
  //ssh_password            = "YOUR_PASSWORD_HERE"
  network_adapters {
    bridge = "vmbr1"
  }
  proxmox_url             = "${var.proxmox_url}"
  token                   = "${var.proxmox_token}"
  username                = "${var.proxmox_username}"
  os                      = "l26"
  qemu_agent              = true
  insecure_skip_tls_verify = true
}

build {
  description = "Tugbot template build"

  sources = ["source.proxmox-iso.tugbot"]

  provisioner "shell" {
    pause_before = "40s"
    max_retries = 5
    inline = [
      "sleep 30",
      "sudo apt-get -y install git build-essential libpq-dev pkg-config libssl-dev",
      "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- --profile minimal -y",
      ". $HOME/.cargo/env",
      "mkdir -p /usr/src/tugbot",
      "git config --global http.sslVerify false",
      "git clone https://github.com/danielcherubini/tugbot-rs.git /usr/src/tugbot",
      "cd /usr/src/tugbot",
      "cargo install --path .",
      "echo 'DISCORD_TOKEN=${var.discord_token}\nAPPLICATION_ID=${var.discord_application_id}\nDATABASE_URL=${var.database_url}' > .env",
      "cp systemd/tugbot.service /etc/systemd/system",
      "systemctl daemon-reload",
      "systemctl enable tugbot.service"
    ]
  }
}
