// Spec 113 — resource route: POST proxy for the clone submit.
//
// Browser-side fetch from the dialog hits this action, which forwards
// the request to Encore via the SSR helper that injects the session
// cookie. Returns the typed `CloneProjectResponse` (or an error JSON).

import { requireUser } from "../lib/auth.server";
import {
  cloneProject,
  type CloneProjectResponse,
} from "../lib/projects-api.server";

interface CloneSubmitBody {
  name?: string;
  slug?: string;
  repoName?: string;
}

export async function action({
  request,
  params,
}: {
  request: Request;
  params: { sourceProjectId: string };
}) {
  await requireUser(request);
  if (request.method !== "POST") {
    return Response.json(
      { error: "method not allowed" },
      { status: 405, headers: { Allow: "POST" } }
    );
  }
  let body: CloneSubmitBody;
  try {
    body = (await request.json()) as CloneSubmitBody;
  } catch {
    return Response.json({ error: "invalid JSON body" }, { status: 400 });
  }
  try {
    const result: CloneProjectResponse = await cloneProject(
      request,
      params.sourceProjectId,
      {
        name: body.name,
        slug: body.slug,
        repoName: body.repoName,
      }
    );
    return Response.json(result);
  } catch (err) {
    return Response.json(
      { error: err instanceof Error ? err.message : String(err) },
      { status: 502 }
    );
  }
}
