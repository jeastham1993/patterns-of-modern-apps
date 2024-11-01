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