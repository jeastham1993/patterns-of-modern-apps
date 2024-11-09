docker-local:
	docker-compose up -d

run-local: docker-local apply-migrations
	
run-local-all:
	docker-compose -f ./docker-compose-all.yml up -d

run-ci:
	docker-compose -f ./docker-compose-dockerhub.yml up -d

apply-migrations:
	export DATABASE_URL=postgresql://postgres:mysupersecretlocalpassword@localhost/loyalty
	sleep 5
	cd src/core;cargo sqlx migrate run
	
integration-test-run:
	export BROKER=localhost:9092;cd integration-tests;cargo test
	docker-compose -f docker-compose-all.yml down

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