variable "ssh_key" {
  default = "$SSH_KEY"
}

variable "node" {
  type = object({
      janus = string
      jove = string
      bacchus = string
    })
  default = {
      janus = "janus"
      jove = "jove"
      bacchus = "bacchus"
    }
}

variable "template" {
  default = "tugbot-template"
}
