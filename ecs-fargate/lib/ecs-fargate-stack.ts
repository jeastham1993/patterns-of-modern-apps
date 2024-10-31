import * as cdk from "aws-cdk-lib";
import { Construct } from "constructs";
import * as ec2 from "aws-cdk-lib/aws-ec2";
import * as ecs from "aws-cdk-lib/aws-ecs";
import { InstrumentedService } from "./instrumented_service";
import { WebService } from "./web_service";
import { StringParameter } from "aws-cdk-lib/aws-ssm";

export class EcsFargateStack extends cdk.Stack {
  constructor(scope: Construct, id: string, props?: cdk.StackProps) {
    super(scope, id, props);

    const vpc = new ec2.Vpc(this, "ModernAppsVpc", {
      maxAzs: 3, // Default is all AZs in region
    });

    const cluster = new ecs.Cluster(this, "LoyaltyCluster", {
      vpc: vpc,
    });

    const databaseUrlParam = StringParameter.fromStringParameterName(
      this,
      "DatabaseUrl",
      "/ModernApps/DatabaseUrl"
    );
    const kafkaBroker = StringParameter.fromStringParameterName(
      this,
      "KafkaBroker",
      "/ModernApps/KafkaBroker"
    );
    const kafkaUsername = StringParameter.fromStringParameterName(
      this,
      "KafkaUsername",
      "/ModernApps/KafkaUsername"
    );
    const kafkaPassword = StringParameter.fromStringParameterName(
      this,
      "KafkaPassword",
      "/ModernApps/KafkaPassword"
    );

    const webService = new WebService(this, "LoyaltyWeb", {
      instrumentService: {
        image: ecs.ContainerImage.fromRegistry(
          "plantpowerjames/modern-apps-loyalty-web:992e550"
        ),
        serviceName: "loyalty-web-fargate",
        environment: "dev",
        version: "latest",
        cluster: cluster,
        vpc: vpc,
        portMappings: [
          {
            containerPort: 8080,
            protocol: ecs.Protocol.TCP,
          },
        ],
        envVariables: {},
        secretVariables: {
          DATABASE_URL: ecs.Secret.fromSsmParameter(databaseUrlParam),
        },
      },
    });

    databaseUrlParam.grantRead(webService.service.executionRole);

    const backendService = new InstrumentedService(this, "LoyaltyBackend", {
      image: ecs.ContainerImage.fromRegistry(
        "plantpowerjames/modern-apps-loyalty-backend:992e550"
      ),
      serviceName: "loyalty-backend-fargate",
      environment: "dev",
      version: "latest",
      cluster: cluster,
      vpc: vpc,
      portMappings: undefined,
      envVariables: {
        GROUP_ID: "loyalty-fargate",
      },
      secretVariables: {
        DATABASE_URL: ecs.Secret.fromSsmParameter(databaseUrlParam),
        BROKER: ecs.Secret.fromSsmParameter(kafkaBroker),
        KAFKA_USERNAME: ecs.Secret.fromSsmParameter(kafkaUsername),
        KAFKA_PASSWORD: ecs.Secret.fromSsmParameter(kafkaPassword),
      },
    });

    databaseUrlParam.grantRead(webService.service.executionRole);
    kafkaBroker.grantRead(webService.service.executionRole);
    kafkaUsername.grantRead(webService.service.executionRole);
    kafkaPassword.grantRead(webService.service.executionRole);
  }
}
