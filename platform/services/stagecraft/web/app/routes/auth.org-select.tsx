/**
 * Org picker for multi-org users (spec 080 FR-012, updated Phase 5).
 *
 * Shown after GitHub login when the user belongs to multiple installed orgs.
 * Org data is stored server-side; the loader fetches it via the pending-orgs
 * API endpoint (Phase 5 removed HMAC-signed cookies).
 */

import { useLoaderData, redirect } from "react-router";

interface OrgOption {
  orgId: string;
  orgSlug: string;
  githubOrgLogin: string;
  platformRole: string;
}

interface LoaderData {
  githubLogin: string;
  orgs: OrgOption[];
}

export async function loader({ request }: { request: Request }) {
  const cookieHeader = request.headers.get("Cookie") || "";
  const apiBase = process.env.ENCORE_API_BASE_URL ?? "http://localhost:4000";

  // Forward the cookie to the API endpoint to resolve pending org data
  const resp = await fetch(`${apiBase}/auth/pending-orgs`, {
    headers: { Cookie: cookieHeader },
  });

  if (!resp.ok) {
    return redirect("/signin?error=session_expired");
  }

  const data = (await resp.json()) as LoaderData;
  if (!data.githubLogin || !data.orgs?.length) {
    return redirect("/signin?error=session_expired");
  }

  return data;
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
