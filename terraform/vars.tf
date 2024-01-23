variable "pm_api_url" {}
variable "pm_api_token_id" {}
variable "pm_api_token_secret" {}
variable "ssh_key" {}

variable "node" {
  type = object({
    janus   = string
    jove    = string
    bacchus = string
  })
  default = {
    janus   = "janus"
    jove    = "jove"
    bacchus = "bacchus"
  }
}

variable "template" {
  default = "tugbot-template-new"
}
