packer {
  required_plugins {
    proxmox = {
      version = ">= 1.1.0"
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
