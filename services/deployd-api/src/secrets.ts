import fs from "node:fs/promises";
import path from "node:path";

export type SecretsReader = {
    read(name: string): Promise<string | null>;
};

export function createFileSecretsReader(dir: string): SecretsReader {
    return {
        async read(name: string) {
            const p = path.join(dir, name);
            try {
                const v = await fs.readFile(p, "utf8");
                return v.trim();
            } catch (err: any) {
                if (err?.code === "ENOENT") return null;
                throw err;
            }
        },
    };
}
