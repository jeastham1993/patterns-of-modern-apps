resource "google_secret_manager_secret" "dd_api_key" {
  secret_id = "dd-api-key"
  replication {
    auto {}
  }
}

resource "google_secret_manager_secret_version" "dd_api_key_version" {
  secret = google_secret_manager_secret.dd_api_key.id

  secret_data = var.dd_api_key
}

resource "google_secret_manager_secret" "database_url" {
  secret_id = "database-url"
  replication {
    auto {}
  }
}

resource "google_secret_manager_secret_version" "database_url_version" {
  secret = google_secret_manager_secret.database_url.id

  secret_data = var.database_url
}

resource "google_secret_manager_secret" "kafka_broker" {
  secret_id = "kafka-broker"
  replication {
    auto {}
  }
}

resource "google_secret_manager_secret_version" "kafka_broker_version" {
  secret = google_secret_manager_secret.kafka_broker.id

  secret_data = var.kafka_broker
}

resource "google_secret_manager_secret" "kafka_username" {
  secret_id = "kafka-username"
  replication {
    auto {}
  }
}

resource "google_secret_manager_secret_version" "kafka_username_version" {
  secret = google_secret_manager_secret.kafka_username.id

  secret_data = var.kafka_username
}

resource "google_secret_manager_secret" "kafka_password" {
  secret_id = "kafka-password"
  replication {
    auto {}
  }
}

resource "google_secret_manager_secret_version" "kafka_password_version" {
  secret = google_secret_manager_secret.kafka_password.id

  secret_data = var.kafka_password
}
