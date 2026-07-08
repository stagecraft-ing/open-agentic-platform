// Spec 227 Stage 2: the pure mapping between the create form's two orthogonal
// axes (Topology x Auth) and the Build Spec `variant` the backend expects, plus
// the manifest profile name a pair resolves to. Auth is not independently
// selectable for `dual` (it serves both stacks); `minimal` (mock auth) is never
// produced at the OAP surface. Kept pure (no react-router/server imports) so
// the mapping is unit-testable and safe in the client bundle.

export type Topology = "single" | "dual";
export type Auth = "public" | "internal";
export type Variant = "single-public" | "single-internal" | "dual";

/** Map the two axes to the Build Spec variant the backend still expects. */
export function toVariant(topology: Topology, auth: Auth): Variant {
  if (topology === "dual") return "dual";
  return auth === "internal" ? "single-internal" : "single-public";
}

/** The manifest profile name a (topology, auth) pair resolves to. */
export function profileName(topology: Topology, auth: Auth): string {
  return topology === "dual" ? "dual" : auth;
}
