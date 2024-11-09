variable "env" {
  type = string
}

variable "app_version" {
  type = string
  default = "latest"
}

variable "dd_site" {
  type = string
}

variable "dd_api_key" {
  type = string
}

variable "subscription_id" {
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
