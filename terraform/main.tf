terraform {
  required_providers {
    proxmox = {
      source = "telmate/proxmox"
      version = "2.7.4"
    }
  }
}

provider "proxmox" {
  pm_api_url = "$PROXMOX_URL"
  pm_api_token_id = "$PROXMOX_USERNAME"
  pm_api_token_secret = "$PROXMOX_TOKEN"
  pm_tls_insecure = true
}
resource "proxmox_vm_qemu" "tugbot" {
  count = 1
  name = "tugbot"
  target_node = var.node.nox
  clone = var.template
  agent = 1
  os_type = "cloud-init"
  cores = 2
  sockets = 1
  cpu = "host"
  memory = 2048
  scsihw = "virtio-scsi-pci"
  bootdisk = "scsi0"
  disk {
    slot = 0
    size = "10G"
    type = "scsi"
    storage = "nox"
    iothread = 1
  }
  
  network {
    model = "virtio"
    bridge = "vmbr0"
  }
  lifecycle {
    ignore_changes = [
      network,
    ]
  }

  ipconfig0 = "ip=dhcp"
  
  sshkeys = <<EOF
  ${var.ssh_key}
  EOF
}

