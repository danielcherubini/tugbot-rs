packer {
  required_plugins {
    proxmox = {
      version = ">= 1.1.0"
      source  = "github.com/hashicorp/proxmox"
    }
  }
}

source "proxmox-iso" "tugbot" {
  boot_command = ["<up><tab> ip=dhcp inst.cmdline inst.ks=http://{{ .HTTPIP }}:{{ .HTTPPort }}/ks.cfg<enter>"]
  boot_wait    = "10s"

  disks {
    disk_size         = "5G"
    storage_pool      = "local-lvm"
    type              = "scsi"
  }
  efi_config {
    efi_storage_pool  = "local-lvm"
    efi_type          = "4m"
    pre_enrolled_keys = true
  }

  http_directory      = "config"
  insecure_skip_tls_verify = true
  iso {
    iso_file          = "local:iso/debian-12-8.0-amd64-netinst.iso"
  }
  network_adapters {
    bridge = "vmbr1"
  }
  node                 = "jove"
  proxmox_url          = "https://my-proxmox.my-domain:8006/api2/json"
  ssh_password         = "packer"
  ssh_timeout          = "15m"
  ssh_username         = "root"
  template_description = "tugbot, generated on ${timestamp()}"
  template_name        = "tugbot"
  token                = "${var.proxmox_token}"
  username             = "${var.proxmox_username}"
  proxmox_url          = "${var.proxmox_url}"

  //ssh_password            = "YOUR_PASSWORD_HERE"
  network_adapters {
    bridge = "vmbr1"
  }
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
