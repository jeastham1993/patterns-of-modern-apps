variable "project_id" {
  type        = string
  description = "Scaleway Project ID"
}

variable "db_connection_string" {
  type        = string
  description = "Database connection string"
}

variable "momento_api_key" {
  type        = string
  description = "Momento API key for caching"
}

variable "cache_name" {
  type        = string
  description = "Name of the cache to use with Momento"
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
