terraform {
  required_providers {
    proxmox = {
      source  = "telmate/proxmox"
      version = "2.9.11"
    }
  }
}

provider "proxmox" {
  pm_api_url          = var.pm_api_url
  pm_api_token_id     = var.pm_api_token_id
  pm_api_token_secret = var.pm_api_token_secret
  pm_tls_insecure     = true
}
resource "proxmox_vm_qemu" "tugbot" {
  count       = 1
  name        = "tugbot-${var.tag_version}"
  target_node = var.node.jove
  clone       = "${var.template}-${var.tag_version}"
  agent       = 1
  os_type     = "cloud-init"
  cores       = 2
  sockets     = 1
  cpu         = "host"
  memory      = 2048
  scsihw      = "virtio-scsi-single"
  bootdisk    = "scsi0"

  disks {
      ide {
          ide3 {
              cloudinit {
                  storage = "local-lvm"
              }
          }
      }
      scsi {
          scsi0 {
              disk {
                  size            = 32
                  storage         = "local-lvm"
                  storage_type    = "scsi"
                  iothread        = true
                  discard         = true
              }
          }
      }
  }

  network {
    model  = "virtio"
    bridge = "vmbr1"
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

