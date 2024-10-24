import * as cdk from "aws-cdk-lib";
import { Construct } from "constructs";
import * as ec2 from "aws-cdk-lib/aws-ec2";
import * as ecs from "aws-cdk-lib/aws-ecs";
import { InstrumentedService } from "./instrumented_service";
import { WebService } from "./web_service";

export class EcsFargateStack extends cdk.Stack {
  constructor(scope: Construct, id: string, props?: cdk.StackProps) {
    super(scope, id, props);

    const vpc = new ec2.Vpc(this, "ModernAppsVpc", {
      maxAzs: 3, // Default is all AZs in region
    });

    const cluster = new ecs.Cluster(this, "LoyaltyCluster", {
      vpc: vpc,
    });

    const webService = new WebService(this, "LoyaltyWeb", {
      instrumentService: {
        image: ecs.ContainerImage.fromRegistry(
          "plantpowerjames/modern-apps-loyalty-web:latest"
        ),
        serviceName: "loyalty-web",
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
          DATABASE_URL: process.env.DATABASE_URL!,
        },
      },
    });

    const backendService = new InstrumentedService(this, "LoyaltyBackend", {
      image: ecs.ContainerImage.fromRegistry(
        "plantpowerjames/modern-apps-loyalty-backend:latest"
      ),
      serviceName: "loyalty-backend",
      environment: "dev",
      version: "latest",
      cluster: cluster,
      vpc: vpc,
      portMappings: undefined,
      envVariables: {
        DATABASE_URL: process.env.DATABASE_URL!,
        BROKER: process.env.BROKER!,
        GROUP_ID: "loyalty-fargate",
        KAFKA_USERNAME: process.env.KAFKA_USERNAME!,
        KAFKA_PASSWORD: process.env.KAFKA_PASSWORD!,
      },
    });
  }
}
