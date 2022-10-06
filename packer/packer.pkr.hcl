variable "discord_application_id" {
  type    = string
  default = "${env("DISCORD_APPLICATION_ID")}"
}

variable "discord_token" {
  type    = string
  default = "${env("DISCORD_TOKEN")}"
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

source "proxmox-clone" "tugbot" {
  clone_vm                 = "debian-11"
  cores                    = 4
  insecure_skip_tls_verify = true
  memory                   = 4096
  network_adapters {
    bridge = "vmbr0"
  }
  node                 = "jove"
  onboot               = true
  os                   = "l26"
  proxmox_url          = "${var.proxmox_url}"
  qemu_agent           = true
  sockets              = 1
  ssh_username         = "root"
  template_description = "image made from cloud-init image"
  template_name        = "tugbot-template"
  token                = "${var.proxmox_token}"
  username             = "${var.proxmox_username}"
}

build {
  description = "Tugbot template build"

  sources = ["source.proxmox-clone.tugbot"]

  provisioner "shell" {
    inline = [
      "sudo apt-get -y install git build-essential",
      "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- --profile minimal -y",
      ". $HOME/.cargo/env",
      "mkdir -p /usr/src/tugbot",
      "git clone https://gitlab.com/danielcherubini/tugbot-rs.git /usr/src/tugbot",
      "cd /usr/src/tugbot",
      "cargo install --path .",
      "echo 'DISCORD_TOKEN=${var.discord_token}\nAPPLICATION_ID=${var.discord_application_id}' > .env",
      "cp systemd/tugbot.service /etc/systemd/system",
      "systemctl daemon-reload",
      "systemctl enable tugbot.service"
    ]
  }
}