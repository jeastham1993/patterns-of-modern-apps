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

variable "env" {
  type = string
}

variable "dd_site" {
  type = string
}

variable "dd_api_key" {
  type = string
}

variable "kafka_broker" {
  type = string
}

variable "kafka_username" {
  type = string
}

variable "kafka_password" {
  type = string
}

variable "database_url" {
  type = string
}

variable "momento_api_key" {
  type    = string
  default = ""
}

variable "momento_cache_name" {
  type    = string
  default = ""
}