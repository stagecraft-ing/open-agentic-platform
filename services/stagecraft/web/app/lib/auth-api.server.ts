/**
 * Auth API helpers using the Encore client.
 * Wraps the generated client for use in loaders/actions with request-scoped base URL.
 */

import { createEncoreClient } from "./encore.server";

export async function authSignin(
  request: Request,
  email: string,
  password: string
) {
  const client = createEncoreClient(request);
  return client.auth.signin({ email, password });
}

export async function authSignup(
  request: Request,
  email: string,
  name: string,
  password: string
) {
  const client = createEncoreClient(request);
  return client.auth.signup({ email, name, password });
}

export async function authAdminSignin(
  request: Request,
  email: string,
  password: string
) {
  const client = createEncoreClient(request);
  return client.auth.adminSignin({ email, password });
}

export async function authSession(request: Request, token: string) {
  const client = createEncoreClient(request);
  return client.auth.session({ token });
}

export async function authAdminSession(request: Request, token: string) {
  const client = createEncoreClient(request);
  return client.auth.adminSession({ token });
}

export async function authSignout(request: Request) {
  const client = createEncoreClient(request);
  return client.auth.signout();
}
