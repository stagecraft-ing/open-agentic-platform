import { createRemoteJWKSet, jwtVerify, type JWTPayload } from "jose";

type CachedConfig = {
    issuer: string;
    jwksUri: string;
    fetchedAtMs: number;
};

let cached: CachedConfig | null = null;

async function fetchOidcConfig(logtoEndpoint: string): Promise<CachedConfig> {
    const url = new URL("/oidc/.well-known/openid-configuration", logtoEndpoint).toString();
    const resp = await fetch(url);
    if (!resp.ok) throw new Error(`Failed to fetch OIDC config: ${resp.status}`);
    const json = (await resp.json()) as { issuer: string; jwks_uri: string };

    return {
        issuer: json.issuer,
        jwksUri: json.jwks_uri,
        fetchedAtMs: Date.now(),
    };
}

async function getConfig(logtoEndpoint: string): Promise<CachedConfig> {
    // Cache for 10 minutes
    if (cached && Date.now() - cached.fetchedAtMs < 10 * 60 * 1000) return cached;
    cached = await fetchOidcConfig(logtoEndpoint);
    return cached;
}

function extractBearer(authHeader: string | undefined): string | null {
    if (!authHeader) return null;
    if (!authHeader.startsWith("Bearer ")) return null;
    const t = authHeader.slice("Bearer ".length).trim();
    return t.length ? t : null;
}

export async function verifyLogtoJwt(opts: {
    authorizationHeader?: string;
    logtoEndpoint: string;
    audience: string;
}): Promise<JWTPayload> {
    const token = extractBearer(opts.authorizationHeader);
    if (!token) throw new Error("Missing bearer token");

    const cfg = await getConfig(opts.logtoEndpoint);

    const jwks = createRemoteJWKSet(new URL(cfg.jwksUri));
    const { payload } = await jwtVerify(token, jwks, {
        issuer: cfg.issuer,
        audience: opts.audience,
    });

    return payload;
}
