import { Construct } from "constructs";
import { IRole, ManagedPolicy, Role, ServicePrincipal } from "aws-cdk-lib/aws-iam";
import { StringParameter } from "aws-cdk-lib/aws-ssm";
import {
  ContainerImage,
  CpuArchitecture,
  FargateService,
  FargateTaskDefinition,
  FirelensLogRouterType,
  ICluster,
  LogDrivers,
  OperatingSystemFamily,
  PortMapping,
  Secret,
} from "aws-cdk-lib/aws-ecs";
import { IVpc } from "aws-cdk-lib/aws-ec2";

export interface InstrumentedServiceProps {
  serviceName: string;
  environment: string;
  version: string;
  image: ContainerImage;
  cluster: ICluster;
  vpc: IVpc;
  portMappings: PortMapping[] | undefined;
  envVariables: {
    [key: string]: string;
  };
  secretVariables: {
    [key: string]: Secret;
  };
}

export class InstrumentedService extends Construct {
  service: FargateService;
  executionRole: IRole;

  constructor(scope: Construct, id: string, props: InstrumentedServiceProps) {
    super(scope, id);

    var ddApiKeyParam = StringParameter.fromStringParameterName(
      this,
      "DDApiKey",
      "DDApiKey"
    );

    this.executionRole = new Role(this, `${props.serviceName}ExecutionRole`, {
      assumedBy: new ServicePrincipal("ecs-tasks.amazonaws.com"),
    });
    this.executionRole.addManagedPolicy(
      ManagedPolicy.fromManagedPolicyArn(
        this,
        "TaskExecutionPolicy",
        "arn:aws:iam::aws:policy/service-role/AmazonECSTaskExecutionRolePolicy"
      )
    );

    var taskRole = new Role(this, `${props.serviceName}TaskRole`, {
      assumedBy: new ServicePrincipal("ecs-tasks.amazonaws.com"),
    });

    var baseEnvironmentVariables: {
      [key: string]: string;
    } = {
      OTLP_ENDPOINT: "http://127.0.0.1:4317",
      Environment: props.environment,
      ECS_ENABLE_CONTAINER_METADATA: "true",
      ENV: props.environment,
      DD_ENV: props.environment,
      SERVICE_NAME: props.serviceName,
      DD_SERVICE: props.serviceName,
      DD_VERSION: props.version,
      DD_IAST_ENABLED: "true",
      RUST_LOG: "info",
      ...props.envVariables,
    };

    var taskDefinition = new FargateTaskDefinition(
      this,
      `${props.serviceName}Definition`,
      {
        memoryLimitMiB: 512,
        runtimePlatform: {
          cpuArchitecture: CpuArchitecture.ARM64,
          operatingSystemFamily: OperatingSystemFamily.LINUX,
        },
        executionRole: this.executionRole,
        taskRole: taskRole,
      }
    );
    var container = taskDefinition.addContainer("application", {
      image: props.image,
      portMappings: props.portMappings,
      containerName: props.serviceName,
      environment: baseEnvironmentVariables,
      secrets: props.secretVariables,
      logging: LogDrivers.firelens({
        options: {
          Name: "datadog",
          Host: "http-intake.logs.datadoghq.eu",
          TLS: "on",
          dd_service: props.serviceName,
          dd_source: "aspnet",
          dd_message_key: "log",
          dd_tags: `project:${props.serviceName}`,
          provider: "ecs",
        },
        secretOptions: {
          apikey: Secret.fromSsmParameter(ddApiKeyParam),
        },
      }),
    });
    container.addDockerLabel("com.datadoghq.tags.env", props.environment);
    container.addDockerLabel("com.datadoghq.tags.service", props.serviceName);
    container.addDockerLabel("com.datadoghq.tags.version", props.version);

    taskDefinition.addContainer("datadog-agent", {
      image: ContainerImage.fromRegistry("public.ecr.aws/datadog/agent:latest"),
      portMappings: [
        {
          containerPort: 4317,
        },
        {
          containerPort: 5000,
        },
        {
          containerPort: 5002,
        },
        {
          containerPort: 8125,
        },
        {
          containerPort: 8126,
        },
      ],
      containerName: "datadog-agent",
      environment: {
        DD_SITE: "datadoghq.eu",
        ECS_FARGATE: "true",
        DD_OTLP_CONFIG_RECEIVER_PROTOCOLS_GRPC_ENDPOINT: "0.0.0.0:4317",
        DD_LOGS_ENABLED: "false",
        DD_DOGSTATSD_NON_LOCAL_TRAFFIC: "true",
        DD_APM_ENABLED: "true",
        DD_APM_NON_LOCAL_TRAFFIC: "true",
        DD_ENV: props.environment,
        DD_SERVICE: props.serviceName,
        DD_VERSION: props.version,
      },
      secrets: {
        DD_API_KEY: Secret.fromSsmParameter(ddApiKeyParam),
      },
    });

    taskDefinition.addFirelensLogRouter("firelens", {
      essential: true,
      image: ContainerImage.fromRegistry("amazon/aws-for-fluent-bit:stable"),
      containerName: "log-router",
      firelensConfig: {
        type: FirelensLogRouterType.FLUENTBIT,
        options: {
          enableECSLogMetadata: true,
        },
      },
    });

    this.service = new FargateService(this, `${props.serviceName}Service`, {
      cluster: props.cluster,
      taskDefinition: taskDefinition,
      desiredCount: 1,
      assignPublicIp: true,
    });

    ddApiKeyParam.grantRead(this.executionRole);
  }
}
