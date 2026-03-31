import { spawn } from "node:child_process";

function run(cmd: string, args: string[], env: Record<string, string | undefined>): Promise<void> {
    return new Promise((resolve, reject) => {
        const p = spawn(cmd, args, { stdio: "inherit", env: { ...process.env, ...env } });
        p.on("exit", (code) => {
            if (code === 0) resolve();
            else reject(new Error(`${cmd} ${args.join(" ")} exited with code ${code}`));
        });
        p.on("error", reject);
    });
}

export async function helmUpsertTenantApp(opts: {
    releaseName: string;
    namespace: string;
    chartPath: string;
    values: {
        appName: string;
        labels: Record<string, string>;
        imageRepository: string;
        imageTag: string;
        ingress?: {
            enabled: boolean;
            hosts?: { host: string; paths: { path: string; pathType: string }[] }[];
            tls?: { secretName: string; hosts: string[] }[];
        };
    };
}): Promise<void> {
    const setArgs: string[] = [];
    const ingress = opts.values.ingress;

    setArgs.push(`app.name=${opts.values.appName}`);
    setArgs.push(`image.repository=${opts.values.imageRepository}`);
    setArgs.push(`image.tag=${opts.values.imageTag}`);

    for (const [k, v] of Object.entries(opts.values.labels)) {
        // Helm set for map keys: app.labels.<key>=<value>
        // Replace / with \. for safety
        const safeKey = k.replace(/\//g, "\\/");
        setArgs.push(`app.labels.${safeKey}=${v}`);
    }

    if (ingress) {
        setArgs.push(`ingress.enabled=${ingress.enabled ? "true" : "false"}`);
        if (ingress.hosts) {
            ingress.hosts.forEach((host, i) => {
                setArgs.push(`ingress.hosts[${i}].host=${host.host}`);
                host.paths.forEach((pathObj, j) => {
                    setArgs.push(`ingress.hosts[${i}].paths[${j}].path=${pathObj.path}`);
                    setArgs.push(`ingress.hosts[${i}].paths[${j}].pathType=${pathObj.pathType}`);
                });
            });
        }
        if (ingress.tls) {
            ingress.tls.forEach((tlsConfig, i) => {
                setArgs.push(`ingress.tls[${i}].secretName=${tlsConfig.secretName}`);
                tlsConfig.hosts.forEach((host, j) => {
                    setArgs.push(`ingress.tls[${i}].hosts[${j}]=${host}`);
                });
            });
        }
    }

    const args = [
        "upgrade",
        "--install",
        opts.releaseName,
        opts.chartPath,
        "--namespace",
        opts.namespace,
        "--create-namespace",
    ];

    for (const kv of setArgs) {
        args.push("--set");
        args.push(kv);
    }

    await run("helm", args, {});
}
