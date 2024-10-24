import { Construct } from "constructs";
import {
  ApplicationListener,
  ApplicationLoadBalancer,
  ApplicationTargetGroup,
  ListenerAction,
  ListenerCondition,
} from "aws-cdk-lib/aws-elasticloadbalancingv2";
import {
  InstrumentedService,
  InstrumentedServiceProps,
} from "./instrumented_service";

export interface WebServiceProps {
  instrumentService: InstrumentedServiceProps;
}

export class WebService extends Construct {
  service: InstrumentedService;
  constructor(scope: Construct, id: string, props: WebServiceProps) {
    super(scope, id);

    this.service = new InstrumentedService(
      this,
      id,
      props.instrumentService
    );

    var targetGroup = new ApplicationTargetGroup(
      this,
      `${props.instrumentService.serviceName}TargetGroup`,
      {
        port: 8080,
        targets: [this.service.service],
        healthCheck: {
          port: "8080",
          path: "/",
          healthyHttpCodes: "200-404",
        },
        vpc: props.instrumentService.vpc,
      }
    );

    const sharedAlbWithListener = new ApplicationLoadBalancer(
      this,
      `${props.instrumentService.serviceName}ApplicationIngressWithListener`,
      {
        vpc: props.instrumentService.vpc,
        internetFacing: true,
      }
    );

    const httpListner = new ApplicationListener(
      this,
      `${props.instrumentService.serviceName}Listener`,
      {
        loadBalancer: sharedAlbWithListener,
        port: 80,
        defaultAction: ListenerAction.fixedResponse(404),
      }
    );

    httpListner.addTargetGroups("ECS", {
      conditions: [ListenerCondition.pathPatterns(["*"])],
      priority: 1,
      targetGroups: [targetGroup],
    });
  }
}
