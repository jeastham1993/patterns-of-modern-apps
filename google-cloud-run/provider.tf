terraform {
  backend "gcs" {
    bucket = "serverless-sandbox-tfstate"
    prefix = "terraform/state"
  }
}

provider "google" {
  project = "serverless-sandbox-429409"
  region  = "europe-west2"
}
