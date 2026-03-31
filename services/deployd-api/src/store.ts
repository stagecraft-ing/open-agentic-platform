export type Lane = "LANE_A" | "LANE_B";
export type DeployStatus =
    | "PENDING"
    | "APPLYING"
    | "ROLLED_OUT"
    | "FAILED"
    | "ROLLED_BACK";

export type DeploymentRecord = {
    deployment_key: string; // app_id|env_id|release_sha
    release_id: string;
    app_id: string;
    env_id: string;
    release_sha: string;
    lane: Lane;
    artifact_ref: string;
    created_at: string;
    status: DeployStatus;
    events: { at: string; type: string; message?: string }[];
    endpoints: string[];
    logs_pointer: string;
};

// In-memory for POC only. Replace with Postgres.
const records = new Map<string, DeploymentRecord>();

export function getByKey(key: string): DeploymentRecord | undefined {
    return records.get(key);
}

export function put(rec: DeploymentRecord): void {
    records.set(rec.deployment_key, rec);
}

export function getByReleaseId(release_id: string): DeploymentRecord | undefined {
    for (const r of records.values()) {
        if (r.release_id === release_id) return r;
    }
    return undefined;
}
