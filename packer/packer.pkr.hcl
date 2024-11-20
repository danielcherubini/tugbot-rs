packer {
  required_plugins {
    proxmox = {
      version = "~> 1"
      source  = "github.com/hashicorp/proxmox"
    }
  }
}

source "proxmox-iso" "tugbot" {
  boot_command   = ["<esc><wait>auto url=http://{{ .HTTPIP }}:{{ .HTTPPort }}/preseed.cfg<enter>"]
  boot_wait    = "10s"

  disks {
    disk_size    = "5G"
    storage_pool = "local-lvm"
    type         = "scsi"
    format       = "raw"
  }
  efi_config {
    efi_storage_pool  = "local-lvm"
    efi_type          = "4m"
    pre_enrolled_keys = true
  }
  boot_iso {
    type = "scsi"
    iso_file = "backup:iso/debian-12.8.0-amd64-netinst.iso"
    unmount = true
    iso_checksum = "sha256:04396d12b0f377958a070c38a923c227832fa3b3e18ddc013936ecf492e9fbb3"
  }
  network_adapters {
    bridge = "vmbr1"
  }

  http_directory           = "packer/config"
  insecure_skip_tls_verify = true
  node                     = "jove"
  proxmox_url              = "${var.proxmox_url}"
  username                 = "${var.proxmox_username}"
  token                    = "${var.proxmox_token}"
  ssh_username             = "root"
  ssh_password             = "packer"
  ssh_timeout              = "15m"
  template_description     = "tugbot, generated on ${timestamp()}"
  template_name            = var.template_name
  qemu_agent               = true
  cloud_init               = true
  cloud_init_storage_pool  = var.storage_pool
  cores                    = 4
  memory                   = "4096"

  cores = 4
  memory = "4096"
}

build {
  description = "Tugbot template build"

  sources = ["source.proxmox-iso.tugbot"]

  provisioner "shell" {
    max_retries = 5
    inline = [
      "apt-get -y install git build-essential libpq-dev pkg-config libssl-dev curl",
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

  # Copy default cloud-init config
  provisioner "file" {
    destination = "/etc/cloud/cloud.cfg"
    source      = "packer/config/cloud.cfg"
  }
}
