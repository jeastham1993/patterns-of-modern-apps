terraform {
  backend "gcs" {
    bucket = "serverless-sandbox-tfstate"
    prefix = "patterns-of-modern-apps/state"
  }
}

provider "google" {
  project = "serverless-sandbox-429409"
  region  = "europe-west2"
}
