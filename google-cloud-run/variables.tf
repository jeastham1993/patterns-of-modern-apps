variable "live_image_tag" {
  type = string
}

variable "canary_image_tag" {
    type = string
}

variable "canary_enabled" {
  description = "Enable the canary"
  type = bool
}

variable "canary_percent" {
  description = "Percentage of traffic to send to the canary"
  type = number
}
variable "force_new_revision" {
  type = bool
  default = false
}