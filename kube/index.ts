import * as k8s from "@pulumi/kubernetes";

const appName = "axum-metrics-rs";
const appImage = "mk10969/" + appName + ":latest";
const appLabels = { app: appName };

const deployment = new k8s.apps.v1.Deployment(appName, {
  spec: {
    selector: { matchLabels: appLabels },
    replicas: 1,
    template: {
      metadata: { labels: appLabels },
      spec: { containers: [{ name: appName, image: appImage }] },
    },
  },
});

// Allocate an IP to the Deployment.
const frontend = new k8s.core.v1.Service(appName, {
  metadata: { labels: deployment.spec.template.metadata.labels },
  spec: {
    type: "ClusterIP",
    // ports: [{ port: 9000, targetPort: 9000, protocol: "TCP" }],
    selector: appLabels,
  },
});

// console.log
frontend.spec.externalIPs.apply<void>((a) => console.log(a));
frontend.spec.clusterIP.apply<void>((a) => console.log(a));

// pulumi stack output
// OUTPUT  VALUE
// ip      localhost
export const ip = frontend.status.loadBalancer.apply<string>(
  (lb) => lb.ingress[0].ip || lb.ingress[0].hostname
);
