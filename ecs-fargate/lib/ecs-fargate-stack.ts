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
      maxAzs: 3,
    });

    const cluster = new ecs.Cluster(this, "LoyaltyCluster", {
      vpc: vpc,
    });

    const databaseUrlParam = new StringParameter(this, "DatabaseUrlParam", {
      parameterName: "/loyalty/database-url",
      stringValue: process.env.DATABASE_URL!,
    });
    const kafkaBroker = new StringParameter(this, "KafkaBrokerParam", {
      parameterName: "/loyalty/broker",
      stringValue: process.env.BROKER!,
    });
    const kafkaUsername = new StringParameter(this, "KafkaUsernameParam", {
      parameterName: "/loyalty/kafka-username",
      stringValue: process.env.KAFKA_USERNAME!,
    });
    const kafkaPassword = new StringParameter(this, "KafkaPasswordParam", {
      parameterName: "/loyalty/kafka-password",
      stringValue: process.env.KAFKA_PASSWORD!,
    });
    const momentoApiKeyParam = new StringParameter(this, "MomentoApiKeyParam", {
      parameterName: "/loyalty/momento-api-key",
      stringValue: process.env.MOMENTO_API_KEY ?? "",
    });

    const imageTag = process.env.IMAGE_TAG ?? "latest";
    const simulatorImageTag = process.env.SIMULATOR_IMAGE_TAG ?? "latest";

    const webService = new WebService(this, "LoyaltyWeb", {
      instrumentService: {
        image: ecs.ContainerImage.fromRegistry(
          `plantpowerjames/modern-apps-loyalty-web:${imageTag}`
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
        envVariables: {
          CACHE_NAME: process.env.CACHE_NAME ?? "",
        },
        secretVariables: {
          DATABASE_URL: ecs.Secret.fromSsmParameter(databaseUrlParam),
          MOMENTO_API_KEY: ecs.Secret.fromSsmParameter(momentoApiKeyParam),
        },
      },
    });

    databaseUrlParam.grantRead(webService.service.executionRole);

    const backendService = new InstrumentedService(this, "LoyaltyBackend", {
      image: ecs.ContainerImage.fromRegistry(
        `plantpowerjames/modern-apps-loyalty-backend:${imageTag}`
      ),
      serviceName: "loyalty-backend-fargate",
      environment: "dev",
      version: "latest",
      cluster: cluster,
      vpc: vpc,
      portMappings: undefined,
      envVariables: {
        GROUP_ID: "loyalty-fargate",
        CACHE_NAME: process.env.CACHE_NAME ?? "",
      },
      secretVariables: {
        DATABASE_URL: ecs.Secret.fromSsmParameter(databaseUrlParam),
        BROKER: ecs.Secret.fromSsmParameter(kafkaBroker),
        KAFKA_USERNAME: ecs.Secret.fromSsmParameter(kafkaUsername),
        KAFKA_PASSWORD: ecs.Secret.fromSsmParameter(kafkaPassword),
        MOMENTO_API_KEY: ecs.Secret.fromSsmParameter(momentoApiKeyParam),
      },
    });

    if ((process.env.DEPLOY_SIMULATOR ?? "") === "Y") {
      const simulator = new InstrumentedService(this, "LoyaltySimulator", {
        image: ecs.ContainerImage.fromRegistry(
          `plantpowerjames/modern-apps-loyalty-simulator:${simulatorImageTag}`
        ),
        serviceName: "loyalty-simulator-fargate",
        environment: "dev",
        version: "latest",
        cluster: cluster,
        vpc: vpc,
        portMappings: undefined,
        envVariables: {
          FARGATE_API_ENDPOINT: `http://${webService.endpoint}`,
          HTTP_REQ_PER_SECOND: "1",
          EVENTS_PER_SECOND: "1",
        },
        secretVariables: {
          BROKER: ecs.Secret.fromSsmParameter(kafkaBroker),
          KAFKA_USERNAME: ecs.Secret.fromSsmParameter(kafkaUsername),
          KAFKA_PASSWORD: ecs.Secret.fromSsmParameter(kafkaPassword),
        },
      });

      kafkaBroker.grantRead(simulator.executionRole);
      kafkaUsername.grantRead(simulator.executionRole);
      kafkaPassword.grantRead(simulator.executionRole);
    }

    databaseUrlParam.grantRead(webService.service.executionRole);
    kafkaBroker.grantRead(webService.service.executionRole);
    kafkaUsername.grantRead(webService.service.executionRole);
    kafkaPassword.grantRead(webService.service.executionRole);
  }
}
