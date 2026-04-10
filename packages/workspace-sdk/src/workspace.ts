/**
 * Workspace domain types (spec 087).
 *
 * The workspace is the unit of identity, governance, collaboration,
 * knowledge intake, and factory execution.
 */

// ---------------------------------------------------------------------------
// Core entities
// ---------------------------------------------------------------------------

export interface Workspace {
  id: string;
  orgId: string;
  name: string;
  slug: string;
  objectStoreBucket: string;
  createdAt: string;
  updatedAt: string;
}

export interface Organization {
  id: string;
  name: string;
  slug: string;
  githubOrgId?: number;
  githubOrgLogin?: string;
  createdAt: string;
  updatedAt: string;
}

export interface Project {
  id: string;
  orgId: string;
  workspaceId: string;
  name: string;
  slug: string;
  description: string;
  createdBy: string;
  createdAt: string;
  updatedAt: string;
}

export interface ProjectRepo {
  id: string;
  projectId: string;
  githubOrg: string;
  repoName: string;
  defaultBranch: string;
  isPrimary: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface Environment {
  id: string;
  projectId: string;
  name: string;
  kind: EnvironmentKind;
  k8sNamespace: string | null;
  autoDeployBranch: string | null;
  requiresApproval: boolean;
  createdAt: string;
  updatedAt: string;
}

export type EnvironmentKind = "preview" | "development" | "staging" | "production";

export interface ProjectMember {
  id: string;
  projectId: string;
  userId: string;
  role: ProjectMemberRole;
  createdAt: string;
  updatedAt: string;
}

export type ProjectMemberRole = "viewer" | "developer" | "deployer" | "admin";

// ---------------------------------------------------------------------------
// Workspace membership
// ---------------------------------------------------------------------------

export interface WorkspaceMember {
  userId: string;
  orgId: string;
  platformRole: PlatformRole;
  githubLogin: string;
  avatarUrl?: string;
}

export type PlatformRole = "owner" | "admin" | "member";

// ---------------------------------------------------------------------------
// Request / Response types for workspace CRUD
// ---------------------------------------------------------------------------

export interface CreateWorkspaceRequest {
  name: string;
  slug: string;
}

export interface UpdateWorkspaceRequest {
  name?: string;
}

export interface ListWorkspacesResponse {
  workspaces: Workspace[];
}

export interface GetWorkspaceResponse {
  workspace: Workspace;
}
