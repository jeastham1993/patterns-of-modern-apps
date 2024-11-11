docker-local:
	docker compose up -d

run-local: docker-local apply-migrations
	
run-local-all:
	docker compose -f ./docker-compose-all.yml up -d

run-ci:
	docker compose -f ./docker-compose-dockerhub.yml up -d

apply-migrations:
	sleep 5
	export DATABASE_URL=postgresql://postgres:mysupersecretlocalpassword@localhost/loyalty;cd src/core;cargo sqlx migrate run
	
integration-test-run:
	export BROKER=localhost:9092;cd integration-tests;cargo test
	docker compose -f docker-compose-all.yml down

integration-test-local: run-local-all apply-migrations integration-test-run

integration-test-ci: run-ci apply-migrations integration-test-run

deploy-ecs:
	cd ecs-fargate;cdk deploy

deploy-lambda:
	sam build
	sam deploy

deploy-cloud-run:
	cd google-cloud-run;terraform init -reconfigure;terraform apply --var-file dev.tfvars

deploy-aca:
	cd azure-container-apps;terraform init;terraform apply --var-file dev.tfvars

deploy-fly:
	fly app create --name loyalty-web
	fly app create --name loyalty-backend
	fly secrets set -a loyalty-web DATABASE_URL="${DATABASE_URL}"
	fly secrets set -a loyalty-backend DATABASE_URL="${DATABASE_URL}"
	fly secrets set -a loyalty-backend BROKER="${BROKER}"
	fly secrets set -a loyalty-backend KAFKA_USERNAME="${KAFKA_USERNAME}"
	fly secrets set -a loyalty-backend KAFKA_PASSWORD="${KAFKA_PASSWORD}"
	fly deploy -c fly-web.toml
	fly deploy -c fly-backend.toml

deploy-fly-simulator:
	fly app create --name loyalty-simulator
	fly secrets set -a loyalty-simulator BROKER="${BROKER}"
	fly secrets set -a loyalty-simulator KAFKA_USERNAME="${KAFKA_USERNAME}"
	fly secrets set -a loyalty-simulator KAFKA_PASSWORD="${KAFKA_PASSWORD}"
	fly deploy -c fly-simulator.toml

cloudflare-worker:
	cd src/web-cloudflare;npx wrangler deploy

cloudflare-database:
	npx wrangler d1 create patterns-of-modern-apps
	cd src/web-cloudflare;npx wrangler d1 execute patterns-of-modern-apps --file=./migrations/schema.sql --remote

cloudflare-queues:
	npx wrangler queues create order-completed
	npx wrangler queues create order-completed-dlq

load:
	cd src/simulator;cargo run

destroy-ecs:
	cd ecs-fargate;cdk destroy

destroy-lambda:
	sam delete

destroy-cloud-run:
	cd google-cloud-run;terraform init -reconfigure;terraform destroy --var-file dev.tfvars

destroy-aca:
	cd azure-container-apps;terraform init;terraform destroy --var-file dev.tfvars

destroy-fly:
	fly apps destroy loyalty-web
	fly apps destroy loyalty-backend