packer {
  required_plugins {
    proxmox = {
      version = "~> 1"
      source  = "github.com/hashicorp/proxmox"
    }
  }
}

source "proxmox-clone" "tugbot" {
  clone_vm                 = "tugbot-template-base"
  cores                    = 4
  insecure_skip_tls_verify = true
  memory                   = 4096

  network_adapters {
    bridge = "vmbr1"
  }

  http_directory           = "packer/config"
  node                     = "jove"
  proxmox_url              = "${var.proxmox_url}"
  username                 = "${var.proxmox_username}"
  token                    = "${var.proxmox_token}"
  ssh_username             = "root"
  ssh_password             = "packer"
  ssh_timeout              = "15m"
  template_description     = "tugbot, generated on ${timestamp()}"
  template_name            = local.template_name
  qemu_agent               = true
  cloud_init               = true
  cloud_init_storage_pool  = "backup"
}

build {
  description = "Tugbot template build"

  sources = ["source.proxmox-iso.tugbot"]

  provisioner "shell" {
    max_retries = 5
    inline = [
      "cd /usr/src/tugbot",
      "git pull",
      "cargo install --path .",
      "echo 'DISCORD_TOKEN=${var.discord_token}\nAPPLICATION_ID=${var.discord_application_id}\nDATABASE_URL=${var.database_url}' > .env",
      "cp systemd/tugbot.service /etc/systemd/system",
      "systemctl daemon-reload",
      "systemctl enable tugbot.service"
    ]
  }
}
