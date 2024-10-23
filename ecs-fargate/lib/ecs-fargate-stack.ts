import * as cdk from "aws-cdk-lib";
import { Construct } from "constructs";
import * as ec2 from "aws-cdk-lib/aws-ec2";
import * as ecs from "aws-cdk-lib/aws-ecs";
import * as ecs_patterns from "aws-cdk-lib/aws-ecs-patterns";

export class EcsFargateStack extends cdk.Stack {
  constructor(scope: Construct, id: string, props?: cdk.StackProps) {
    super(scope, id, props);

    const vpc = new ec2.Vpc(this, "ModernAppsVpc", {
      maxAzs: 3, // Default is all AZs in region
    });

    const cluster = new ecs.Cluster(this, "LoyaltyCluster", {
      vpc: vpc,
    });

    // Create a load-balanced Fargate service and make it public
    const service = new ecs_patterns.ApplicationLoadBalancedFargateService(
      this,
      "LoyaltyWebClusters",
      {
        cluster: cluster, // Required
        cpu: 256,
        desiredCount: 1,
        taskImageOptions: {
          image: ecs.ContainerImage.fromRegistry(
            "plantpowerjames/modern-apps-loyalty-web:latest"
          ),
          environment: {
            DATABASE_URL: process.env.DATABASE_URL!
          },
          containerPort: 8080,
        },
        memoryLimitMiB: 512,
        publicLoadBalancer: true, // Default is true
      }
    );

    service.targetGroup.configureHealthCheck({
      port: "8080",
      path: "/",
      healthyHttpCodes: "200-404",
    });

    const backendService = new ecs_patterns.QueueProcessingFargateService(
      this,
      "LoyaltyBackendService",
      {
        cluster: cluster,
        cpu: 256,
        image: ecs.ContainerImage.fromRegistry(
          "plantpowerjames/modern-apps-loyalty-backend:latest"
        ),
        environment: {
          DATABASE_URL: process.env.DATABASE_URL!,
          BROKER: process.env.BROKER!,
          GROUP_ID: "loyalty-fargate",
          KAFKA_USERNAME: process.env.KAFKA_USERNAME!,
          KAFKA_PASSWORD: process.env.KAFKA_PASSWORD!,
        },
        memoryLimitMiB: 512,
      }
    )
  }
}
