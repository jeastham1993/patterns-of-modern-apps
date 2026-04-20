resource "scaleway_container_namespace" "main" {
  name        = "patterns-of-modern-apps"
  description = "Patterns of Modern Apps"
}

resource "scaleway_container" "web-api" {
  name           = "patterns-of-modern-apps-web-api"
  description    = "Web API for patterns of modern apps"
  namespace_id   = scaleway_container_namespace.main.id
  registry_image = "docker.io/plantpowerjames/modern-apps-loyalty-web:latest"
  port           = 8080
  cpu_limit      = 1024
  memory_limit   = 2048
  min_scale      = 1
  max_scale      = 1
  timeout        = 600
  privacy        = "public"
  protocol       = "http1"
  deploy         = true
  environment_variables = {
    "SERVICE_NAME"    = "patterns-of-modern-apps-web-api"
    "DATABASE_URL"    = var.db_connection_string
    "MOMENTO_API_KEY" = var.momento_api_key
    "CACHE_NAME" : var.cache_name,
    "OTLP_ENDPOINT" : ""
  }
}

resource "scaleway_container" "backend-worker" {
  name           = "patterns-of-modern-apps-backend-worker"
  description    = "Backend worker for patterns of modern apps"
  namespace_id   = scaleway_container_namespace.main.id
  registry_image = "docker.io/plantpowerjames/modern-apps-loyalty-backend:latest"
  port           = 8080
  cpu_limit      = 1024
  memory_limit   = 2048
  min_scale      = 1
  max_scale      = 1
  timeout        = 600
  privacy        = "public"
  protocol       = "http1"
  deploy         = true
  environment_variables = {
    "SERVICE_NAME"    = "patterns-of-modern-apps-backend-worker"
    "DATABASE_URL"    = var.db_connection_string
    "MOMENTO_API_KEY" = var.momento_api_key
    "BROKER"          = var.kafka_broker
    "KAFKA_USERNAME"  = var.kafka_username
    "KAFKA_PASSWORD"  = var.kafka_password
    "GROUP_ID"        = "loyalty-scaleway"
    "CACHE_NAME" : var.cache_name,
    "OTLP_ENDPOINT" : ""
  }
}
