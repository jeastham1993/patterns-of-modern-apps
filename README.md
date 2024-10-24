# Patterns of Modern App Development

## AWS

TODO: Docs on secret setup

### Deploy ECS Fargate

```sh
cd ecs-fargate
cdk deploy
```

### Deploy Lambda

```sh
sam build
sam deploy --guided
```

## Azure

Create dev.tfvars file

```tf
env             = ""
dd_site         = ""
dd_api_key      = ""
subscription_id = ""
database_url    = ""
kafka_broker    = ""
kafka_username  = ""
kafka_password  = ""
```

Then deploy

```sh
cd azure-container-apps
az login
terraform init
terraform apply --var-file dev.tfvars
```

## GCP

## Cloud Run

```sh

```

## Fly.IO

```sh
fly app create --name loyalty-web
fly app create --name loyalty-backend
fly deploy
```

### Secrets

```sh
fly secrets set -a loyalty-web DATABASE_URL=""
fly secrets set -a loyalty-backend DATABASE_URL=""
fly secrets set -a loyalty-backend BROKER=""
fly secrets set -a loyalty-backend KAFKA_USERNAME=""
fly secrets set -a loyalty-backend KAFKA_PASSWORD=""
```
