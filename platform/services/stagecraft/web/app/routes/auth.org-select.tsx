/**
 * Org picker for multi-org users (spec 080 FR-012).
 * Shown after GitHub login when the user belongs to multiple installed orgs.
 */

import { useLoaderData, redirect } from "react-router";
import { createHmac, timingSafeEqual } from "crypto";

interface OrgOption {
  orgId: string;
  orgSlug: string;
  githubOrgLogin: string;
  platformRole: string;
}

// See auth.server.ts for SESSION_SECRET access pattern documentation.
function verifyPendingCookie(signed: string): { githubLogin: string; orgs: OrgOption[] } | null {
  const sessionSecret = process.env.SESSION_SECRET;
  if (!sessionSecret) return null;

  const dotIdx = signed.lastIndexOf(".");
  if (dotIdx === -1) return null;

  const payload = signed.substring(0, dotIdx);
  const sig = signed.substring(dotIdx + 1);

  const expected = createHmac("sha256", sessionSecret).update(payload).digest("base64url");
  try {
    if (!timingSafeEqual(Buffer.from(sig), Buffer.from(expected))) return null;
  } catch {
    return null;
  }

  try {
    return JSON.parse(Buffer.from(payload, "base64url").toString());
  } catch {
    return null;
  }
}

export async function loader({ request }: { request: Request }) {
  const cookieHeader = request.headers.get("Cookie") || "";
  const match = cookieHeader.match(/(?:^|;\s*)__pending_org=([^\s;]+)/);

  if (!match) {
    return redirect("/signin?error=session_expired");
  }

  const data = verifyPendingCookie(match[1]);
  if (!data || !data.githubLogin || !data.orgs) {
    return redirect("/signin?error=session_expired");
  }

  return { githubLogin: data.githubLogin, orgs: data.orgs };
}

export default function OrgSelect() {
  const { githubLogin, orgs } = useLoaderData<typeof loader>();

  return (
    <div className="min-h-full container px-4 mx-auto my-16 max-w-sm">
      <h1 className="text-2xl font-bold text-gray-900 dark:text-gray-100">
        Select organization
      </h1>
      <p className="mt-2 text-sm text-gray-600 dark:text-gray-400">
        Welcome, <strong>{githubLogin}</strong>. You belong to multiple
        organizations. Select one to continue.
      </p>
      <div className="mt-6 space-y-3">
        {orgs.map((org) => (
          <a
            key={org.orgId}
            href={`/auth/org-select/complete?org=${encodeURIComponent(org.orgId)}`}
            className="block w-full rounded-lg border border-gray-200 p-4 hover:border-indigo-500 hover:bg-indigo-50 dark:border-gray-700 dark:hover:border-indigo-400 dark:hover:bg-gray-800 transition-colors"
          >
            <div className="flex items-center justify-between">
              <div>
                <p className="font-medium text-gray-900 dark:text-gray-100">
                  {org.githubOrgLogin}
                </p>
                <p className="text-xs text-gray-500 dark:text-gray-400">
                  Role: {org.platformRole}
                </p>
              </div>
              <svg
                className="h-5 w-5 text-gray-400"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M9 5l7 7-7 7"
                />
              </svg>
            </div>
          </a>
        ))}
      </div>
    </div>
  );
}
