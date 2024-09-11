packer {
  required_plugins {
    proxmox = {
      version = ">= 1.1.0"
      source  = "github.com/hashicorp/proxmox"
    }
  }
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
  token                = "${var.proxmox_token}"
  username             = "${var.proxmox_username}"
  qemu_agent           = true
  sockets              = 1
  ssh_username         = "root"
  template_description = "image made from cloud-init image"
  template_name        = "${local.template_name}"
 }

build {
  description = "Tugbot template build"

  sources = ["source.proxmox-clone.tugbot"]

  provisioner "shell" {
    pause_before = "30s"
    max_retries = 5
    inline = [
      "sleep 30",
      "sudo apt-get -y install git build-essential libpq-dev pkg-config",
      "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- --profile minimal -y",
      ". $HOME/.cargo/env",
      "mkdir -p /usr/src/tugbot",
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
