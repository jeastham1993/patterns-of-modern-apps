resource "random_string" "rev_name_postfix_live" {
  # it gets updates on changes to the following 'keepers' - properties of a service
  keepers = {
    image_name         = var.live_image_tag
    force_new_revision = var.force_new_revision
  }
  length  = 2
  special = false
  upper   = false
}

resource "random_string" "rev_name_postfix_canary" {
  keepers = {
    canary_enabled     = var.canary_enabled
    canary_image_name  = var.canary_image_tag
    force_new_revision = var.force_new_revision
  }
  length  = 2
  special = false
  upper   = false
}

locals {
  rev_name_live           = "loyalty-web-${random_string.rev_name_postfix_live.result}"
  rev_name_canary         = "loyalty-web-${random_string.rev_name_postfix_canary.result}"
  backend_rev_name_live   = "loyalty-backend-${random_string.rev_name_postfix_live.result}"
  backend_rev_name_canary = "loyalty-backend-${random_string.rev_name_postfix_canary.result}"
}

resource "google_service_account" "cloudrun_service_identity" {
  account_id = "loyalty-service-account"
}

resource "google_cloud_run_v2_service" "loyalty_web" {
  name     = "loyalty-web"
  location = "europe-west2"
  ingress  = "INGRESS_TRAFFIC_ALL"

  template {
    revision        = var.canary_enabled ? local.rev_name_canary : local.rev_name_live
    service_account = google_service_account.cloudrun_service_identity.email
    containers {
      image = var.canary_enabled ? "docker.io/plantpowerjames/modern-apps-loyalty-web:${var.canary_image_tag}" : "docker.io/plantpowerjames/modern-apps-loyalty-web:${var.live_image_tag}"
      env {
        name  = "SERVICE_NAME"
        value = "loyalty-web-gcp"
      }
      env {
        name = "DATABASE_URL"
        value_source {
          secret_key_ref {
            secret  = google_secret_manager_secret.database_url.id
            version = google_secret_manager_secret_version.database_url_version.version
          }
        }
      }
      env {
        name  = "OTLP_ENDPOINT"
        value = "http://localhost:4317"
      }
      env {
        name  = "GCLOUD_PROJECT_ID"
        value = data.google_project.project.project_id
      }
      dynamic "env" {
        for_each = var.canary_enabled ? { "CANARY" = 1 } : {}
        content {
          name  = env.key
          value = env.value
        }
      }
    }
    containers {
      name  = "datadog"
      image = "gcr.io/datadoghq/serverless-init:latest"
      env {
        name  = "DD_ENV"
        value = var.env
      }
      env {
        name  = "DD_SITE"
        value = var.dd_site
      }
      env {
        name = "DD_API_KEY"
        value_source {
          secret_key_ref {
            secret  = google_secret_manager_secret.dd_api_key.id
            version = "1"
          }
        }
      }
      env {
        name  = "DD_VERSION"
        value = var.canary_enabled ? var.canary_image_tag : var.live_image_tag
      }
      env {
        name  = "GCLOUD_PROJECT_ID"
        value = data.google_project.project.project_id
      }
      env {
        name  = "DD_OTLP_CONFIG_RECEIVER_PROTOCOLS_GRPC_ENDPOINT"
        value = "0.0.0.0:4317"
      }
    }
  }

  traffic {
    type = "TRAFFIC_TARGET_ALLOCATION_TYPE_REVISION"
    # live serves 100% by default. If canary is enabled, this traffic block controls canary
    percent = var.canary_enabled ? var.canary_percent : 100
    # revision is named live by default. When canary is enabled, a new revision named canary is deployed
    revision = var.canary_enabled ? local.rev_name_canary : local.rev_name_live
    tag      = var.canary_enabled ? local.rev_name_canary : local.rev_name_live
  }

  dynamic "traffic" {
    # if canary is enabled, add another traffic block
    for_each = var.canary_enabled ? ["canary"] : []
    content {
      # current live's traffic is now controlled here
      percent  = var.canary_enabled ? 100 - var.canary_percent : 0
      revision = var.canary_enabled ? local.rev_name_live : local.rev_name_canary
      type     = "TRAFFIC_TARGET_ALLOCATION_TYPE_REVISION"
    }
  }
}

data "google_iam_policy" "noauth" {
  binding {
    role = "roles/run.invoker"
    members = [
      "allUsers",
    ]
  }
}

resource "google_cloud_run_service_iam_policy" "noauth" {
  location = google_cloud_run_v2_service.loyalty_web.location
  project  = google_cloud_run_v2_service.loyalty_web.project
  service  = google_cloud_run_v2_service.loyalty_web.name

  policy_data = data.google_iam_policy.noauth.policy_data
}

resource "google_cloud_run_v2_service" "loyalty_backend" {
  name     = "loyalty-backend"
  location = "europe-west2"

  template {
    revision        = var.canary_enabled ? local.backend_rev_name_canary : local.backend_rev_name_live
    service_account = google_service_account.cloudrun_service_identity.email
    containers {
      image = var.canary_enabled ? "docker.io/plantpowerjames/modern-apps-loyalty-backend:${var.canary_image_tag}" : "docker.io/plantpowerjames/modern-apps-loyalty-backend:${var.live_image_tag}"
      env {
        name  = "SERVICE_NAME"
        value = "loyalty-backend-gcp"
      }
      env {
        name = "DATABASE_URL"
        value_source {
          secret_key_ref {
            secret  = google_secret_manager_secret.database_url.id
            version = google_secret_manager_secret_version.database_url_version.version
          }
        }
      }
      env {
        name = "BROKER"
        value_source {
          secret_key_ref {
            secret  = google_secret_manager_secret.kafka_broker.id
            version = "1"
          }
        }
      }
      env {
        name  = "GROUP_ID"
        value = "loyalty-gcp"
      }
      env {
        name = "KAFKA_USERNAME"
        value_source {
          secret_key_ref {
            secret  = google_secret_manager_secret.kafka_username.id
            version = "1"
          }
        }
      }
      env {
        name = "KAFKA_PASSWORD"
        value_source {
          secret_key_ref {
            secret  = google_secret_manager_secret.kafka_password.id
            version = "1"
          }
        }
      }
      env {
        name  = "OTLP_ENDPOINT"
        value = "http://localhost:4317"
      }
      env {
        name  = "GCLOUD_PROJECT_ID"
        value = data.google_project.project.project_id
      }
      dynamic "env" {
        for_each = var.canary_enabled ? { "CANARY" = 1 } : {}
        content {
          name  = env.key
          value = env.value
        }
      }
    }
    containers {
      name  = "datadog"
      image = "gcr.io/datadoghq/serverless-init:latest"
      env {
        name  = "DD_ENV"
        value = var.env
      }
      env {
        name  = "DD_SITE"
        value = var.dd_site
      }
      env {
        name = "DD_API_KEY"
        value_source {
          secret_key_ref {
            secret  = google_secret_manager_secret.dd_api_key.id
            version = "1"
          }
        }
      }
      env {
        name  = "DD_VERSION"
        value = var.canary_enabled ? var.canary_image_tag : var.live_image_tag
      }
      env {
        name  = "GCLOUD_PROJECT_ID"
        value = data.google_project.project.project_id
      }
      env {
        name  = "DD_OTLP_CONFIG_RECEIVER_PROTOCOLS_GRPC_ENDPOINT"
        value = "0.0.0.0:4317"
      }
    }
    scaling {
      min_instance_count = 1
      max_instance_count = 2
    }

    annotations = {
      "run.googleapis.com/cpu-throttling" = false
    }
  }

  traffic {
    type = "TRAFFIC_TARGET_ALLOCATION_TYPE_REVISION"
    # live serves 100% by default. If canary is enabled, this traffic block controls canary
    percent = var.canary_enabled ? var.canary_percent : 100
    # revision is named live by default. When canary is enabled, a new revision named canary is deployed
    revision = var.canary_enabled ? local.backend_rev_name_canary : local.backend_rev_name_live
    tag      = var.canary_enabled ? local.backend_rev_name_canary : local.backend_rev_name_live
  }

  dynamic "traffic" {
    # if canary is enabled, add another traffic block
    for_each = var.canary_enabled ? ["canary"] : []
    content {
      # current live's traffic is now controlled here
      percent  = var.canary_enabled ? 100 - var.canary_percent : 0
      revision = var.canary_enabled ? local.backend_rev_name_canary : local.backend_rev_name_live
      type     = "TRAFFIC_TARGET_ALLOCATION_TYPE_REVISION"
    }
  }
}

resource "google_secret_manager_secret_iam_member" "dd-secret-access" {
  secret_id = google_secret_manager_secret.dd_api_key.id
  role      = "roles/secretmanager.secretAccessor"
  member    = "serviceAccount:${google_service_account.cloudrun_service_identity.email}"
}

resource "google_secret_manager_secret_iam_member" "db-secret-access" {
  secret_id = google_secret_manager_secret.database_url.id
  role      = "roles/secretmanager.secretAccessor"
  member    = "serviceAccount:${google_service_account.cloudrun_service_identity.email}"
}

resource "google_secret_manager_secret_iam_member" "broker-secret-access" {
  secret_id = google_secret_manager_secret.kafka_broker.id
  role      = "roles/secretmanager.secretAccessor"
  member    = "serviceAccount:${google_service_account.cloudrun_service_identity.email}"
}

resource "google_secret_manager_secret_iam_member" "username-secret-access" {
  secret_id = google_secret_manager_secret.kafka_username.id
  role      = "roles/secretmanager.secretAccessor"
  member    = "serviceAccount:${google_service_account.cloudrun_service_identity.email}"
}

resource "google_secret_manager_secret_iam_member" "password-secret-access" {
  secret_id = google_secret_manager_secret.kafka_password.id
  role      = "roles/secretmanager.secretAccessor"
  member    = "serviceAccount:${google_service_account.cloudrun_service_identity.email}"
}
