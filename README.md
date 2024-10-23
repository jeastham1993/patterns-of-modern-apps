# Patterns of Modern App Development

## Deploy Fly.IO

```sh
fly app create --name loyalty-web
fly app create --name loyalty-backend
fly deploy
```

## Secrets

```sh
fly secrets set -a loyalty-web DATABASE_URL=""
fly secrets set -a loyalty-backend DATABASE_URL=""
fly secrets set -a loyalty-backend BROKER=""
fly secrets set -a loyalty-backend KAFKA_USERNAME=""
fly secrets set -a loyalty-backend KAFKA_PASSWORD=""
```