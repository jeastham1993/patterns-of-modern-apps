resource "azurerm_resource_group" "modern_apps_container_apps" {
  name     = "modern-apps-loyalty-${var.env}"
  location = "West Europe"
  tags = {
    source = "terraform"
    env    = var.env
  }
}

resource "azurerm_log_analytics_workspace" "modern_apps_container_apps_log_analytics" {
  name                = "modern-apps-logs-${var.env}"
  location            = azurerm_resource_group.modern_apps_container_apps.location
  resource_group_name = azurerm_resource_group.modern_apps_container_apps.name
  sku                 = "PerGB2018"
  retention_in_days   = 30
  tags = {
    source = "terraform"
    env    = var.env
  }
}

resource "azurerm_container_app_environment" "modern_apps_container_apps_dev_environment" {
  name                       = var.env
  location                   = azurerm_resource_group.modern_apps_container_apps.location
  resource_group_name        = azurerm_resource_group.modern_apps_container_apps.name
  log_analytics_workspace_id = azurerm_log_analytics_workspace.modern_apps_container_apps_log_analytics.id
  tags = {
    source = "terraform"
    env    = var.env
  }
}

resource "azurerm_container_app" "loyalty_web" {
  name                         = "loyalty-web"
  container_app_environment_id = azurerm_container_app_environment.modern_apps_container_apps_dev_environment.id
  resource_group_name          = azurerm_resource_group.modern_apps_container_apps.name
  revision_mode                = "Single"
  identity {
    identity_ids = [azurerm_user_assigned_identity.loyalty_app_identity.id]
    type         = "UserAssigned"
  }
  ingress {
    external_enabled = true
    target_port      = 8080
    traffic_weight {
      percentage      = 100
      latest_revision = true
    }
  }
  template {
    min_replicas = 1
    max_replicas = 1
    container {
      name   = "loyalty-web"
      image  = "plantpowerjames/modern-apps-loyalty-web:${var.app_version}"
      cpu    = 0.25
      memory = "0.5Gi"

      env {
        name  = "DATABASE_URL"
        value = var.database_url
      }
      env {
        name  = "DD_TRACE_OTEL_ENABLED"
        value = "true"
      }
      env {
        name  = "DD_RUNTIME_METRICS_ENABLED"
        value = "true"
      }
      env {
        name  = "DD_LOGS_INJECTION"
        value = "true"
      }
      env {
        name  = "DD_ENV"
        value = var.env
      }
      env {
        name  = "OTLP_ENDPOINT"
        value = "http://localhost:4317"
      }
      env {
        name  = "SERVICE_NAME"
        value = "loyalty-web-aca"
      }
    }
    container {
      name   = "datadog"
      image  = "index.docker.io/datadog/serverless-init:latest"
      cpu    = 0.25
      memory = "0.5Gi"

      env {
        name  = "DD_SITE"
        value = var.dd_site
      }
      env {
        name  = "DD_ENV"
        value = var.env
      }
      env {
        name  = "DD_API_KEY"
        value = var.dd_api_key
      }
      env {
        name  = "DD_VERSION"
        value = var.app_version
      }
      env {
        name  = "DD_AZURE_SUBSCRIPTION_ID"
        value = data.azurerm_subscription.primary.subscription_id
      }
      env {
        name  = "DD_AZURE_RESOURCE_GROUP"
        value = azurerm_resource_group.modern_apps_container_apps.name
      }
      env {
        name  = "DD_OTLP_CONFIG_RECEIVER_PROTOCOLS_GRPC_ENDPOINT"
        value = "0.0.0.0:4317"
      }
      env {
        name  = "DD_APM_IGNORE_RESOURCES"
        value = "/opentelemetry.proto.collector.trace.v1.TraceService/Export$"
      }
    }
  }
}


resource "azurerm_container_app" "loyalty_backend" {
  name                         = "loyalty-backend"
  container_app_environment_id = azurerm_container_app_environment.modern_apps_container_apps_dev_environment.id
  resource_group_name          = azurerm_resource_group.modern_apps_container_apps.name
  revision_mode                = "Single"
  identity {
    identity_ids = [azurerm_user_assigned_identity.loyalty_app_identity.id]
    type         = "UserAssigned"
  }
  ingress {
    external_enabled = true
    target_port      = 8080
    traffic_weight {
      percentage      = 100
      latest_revision = true
    }
  }
  template {
    min_replicas = 1
    max_replicas = 1
    container {
      name   = "loyalty-web"
      image  = "plantpowerjames/modern-apps-loyalty-backend:${var.app_version}"
      cpu    = 0.25
      memory = "0.5Gi"

      env {
        name  = "DATABASE_URL"
        value = var.database_url
      }

      env {
        name  = "BROKER"
        value = var.kafka_broker
      }

      env {
        name  = "GROUP_ID"
        value = var.database_url
      }

      env {
        name  = "KAFKA_USERNAME"
        value = var.kafka_username
      }

      env {
        name  = "KAFKA_PASSWORD"
        value = var.kafka_password
      }
      env {
        name  = "DD_TRACE_OTEL_ENABLED"
        value = "true"
      }
      env {
        name  = "DD_RUNTIME_METRICS_ENABLED"
        value = "true"
      }
      env {
        name  = "DD_LOGS_INJECTION"
        value = "true"
      }
      env {
        name  = "DD_ENV"
        value = var.env
      }
      env {
        name  = "OTLP_ENDPOINT"
        value = "http://localhost:4317"
      }
      env {
        name  = "SERVICE_NAME"
        value = "loyalty-backend-aca"
      }
    }
    container {
      name   = "datadog"
      image  = "index.docker.io/datadog/serverless-init:latest"
      cpu    = 0.25
      memory = "0.5Gi"

      env {
        name  = "DD_SITE"
        value = var.dd_site
      }
      env {
        name  = "DD_ENV"
        value = var.env
      }
      env {
        name  = "DD_API_KEY"
        value = var.dd_api_key
      }
      env {
        name  = "DD_VERSION"
        value = var.app_version
      }
      env {
        name  = "DD_AZURE_SUBSCRIPTION_ID"
        value = data.azurerm_subscription.primary.subscription_id
      }
      env {
        name  = "DD_AZURE_RESOURCE_GROUP"
        value = azurerm_resource_group.modern_apps_container_apps.name
      }
      env {
        name  = "DD_OTLP_CONFIG_RECEIVER_PROTOCOLS_GRPC_ENDPOINT"
        value = "0.0.0.0:4317"
      }
      env {
        name  = "DD_APM_IGNORE_RESOURCES"
        value = "/opentelemetry.proto.collector.trace.v1.TraceService/Export$"
      }
    }
  }
}
