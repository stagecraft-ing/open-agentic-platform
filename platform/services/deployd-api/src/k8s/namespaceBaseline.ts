import * as k8s from "@kubernetes/client-node";

export async function ensureNamespaceWithBaseline(opts: {
    namespace: string;
    labels: Record<string, string>;
}): Promise<void> {
    const kc = new k8s.KubeConfig();
    kc.loadFromCluster();

    const core = kc.makeApiClient(k8s.CoreV1Api);
    const net = kc.makeApiClient(k8s.NetworkingV1Api);

    const nsName = opts.namespace;

    // Ensure namespace
    try {
      await core.readNamespace({ name: nsName });
    } catch (e: any) {
        if (e?.response?.statusCode !== 404) throw e;
        await core.createNamespace({
            metadata: { name: nsName, labels: opts.labels },
        } as any);
    }

    // Ensure default deny network policy
    const npName = "default-deny-all";
    try {
      await net.readNamespacedNetworkPolicy({ name: npName, namespace: nsName });
    } catch (e: any) {
        if (e?.response?.statusCode !== 404) throw e;
        await net.createNamespacedNetworkPolicy({
          namespace: nsName,
          body: {
            metadata: {name: npName},
            spec: {podSelector: {}, policyTypes: ["Ingress", "Egress"]},
          },
        });
    }
}
