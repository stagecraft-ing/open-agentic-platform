import { useState } from "react";
import { Form, Link, redirect, useActionData, useSearchParams } from "react-router";
import { authSignin } from "../lib/auth-api.server";
import { getFormValues } from "../lib/form-data.server";

const ERROR_MESSAGES: Record<string, string> = {
  github_denied: "GitHub login was denied. Please try again.",
  no_email: "Could not retrieve your email from GitHub. Please ensure your GitHub email is verified.",
  token_failed: "GitHub authentication failed. Please try again.",
  github_api_failed: "Could not reach GitHub. Please try again in a moment.",
  account_error: "Failed to create or link your account. Please try again or contact support.",
  membership_failed: "Could not resolve your organization memberships. Please try again.",
  rauthy_unavailable: "Identity service is temporarily unavailable. Please try again later.",
  oauth_failed: "GitHub login failed. Please try again.",
  session_expired: "Session expired. Please sign in again.",
};

export async function action({ request }: { request: Request }) {
  const data = await getFormValues(request);
  const email = String(data.email || "");
  const password = String(data.password || "");

  const res = await authSignin(request, email, password);
  if (!res.ok) return { error: res.error || "Sign-in failed" };

  return redirect("/app", {
    headers: { "Set-Cookie": res.setCookie! },
  });
}

export default function Signin() {
  const data = useActionData() as { error?: string } | undefined;
  const [searchParams] = useSearchParams();
  const oauthError = searchParams.get("error");
  const [showEmailLogin, setShowEmailLogin] = useState(false);

  const errorMessage =
    data?.error || (oauthError ? ERROR_MESSAGES[oauthError] ?? oauthError : null);

  return (
    <div className="min-h-full container px-4 mx-auto my-16 max-w-sm">
      <h1 className="text-2xl font-bold text-gray-900 dark:text-gray-100">
        Sign in
      </h1>
      <p className="mt-2 text-sm text-gray-600 dark:text-gray-400">
        Sign in with your GitHub account to access your organization.
      </p>
      {errorMessage ? (
        <p className="mt-2 text-sm text-red-600 dark:text-red-400">
          {errorMessage}
        </p>
      ) : null}

      {/* GitHub OAuth — primary login */}
      <a
        href="/auth/github"
        className="mt-6 w-full inline-flex items-center justify-center gap-2 rounded-md border border-gray-300 bg-gray-900 px-4 py-2.5 text-sm font-medium text-white shadow-sm hover:bg-gray-800 focus:outline-none focus:ring-2 focus:ring-gray-500 focus:ring-offset-2 dark:border-gray-600 dark:bg-gray-800 dark:hover:bg-gray-700 dark:focus:ring-offset-gray-900"
      >
        <svg className="h-5 w-5" fill="currentColor" viewBox="0 0 24 24">
          <path
            fillRule="evenodd"
            d="M12 2C6.477 2 2 6.484 2 12.017c0 4.425 2.865 8.18 6.839 9.504.5.092.682-.217.682-.483 0-.237-.008-.868-.013-1.703-2.782.605-3.369-1.343-3.369-1.343-.454-1.158-1.11-1.466-1.11-1.466-.908-.62.069-.608.069-.608 1.003.07 1.531 1.032 1.531 1.032.892 1.53 2.341 1.088 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.113-4.555-4.951 0-1.093.39-1.988 1.029-2.688-.103-.253-.446-1.272.098-2.65 0 0 .84-.27 2.75 1.026A9.564 9.564 0 0112 6.844c.85.004 1.705.115 2.504.337 1.909-1.296 2.747-1.027 2.747-1.027.546 1.379.202 2.398.1 2.651.64.7 1.028 1.595 1.028 2.688 0 3.848-2.339 4.695-4.566 4.943.359.309.678.92.678 1.855 0 1.338-.012 2.419-.012 2.747 0 .268.18.58.688.482A10.019 10.019 0 0022 12.017C22 6.484 17.522 2 12 2z"
            clipRule="evenodd"
          />
        </svg>
        Continue with GitHub
      </a>

      {/* Email/password fallback (hidden by default per NFR-004) */}
      {!showEmailLogin ? (
        <button
          type="button"
          onClick={() => setShowEmailLogin(true)}
          className="mt-6 w-full text-center text-sm text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300"
        >
          Sign in with email instead
        </button>
      ) : (
        <>
          <div className="mt-6 border-t border-gray-200 dark:border-gray-700 pt-4">
            <p className="text-xs text-gray-500 dark:text-gray-400 mb-3">
              Email sign-in is available for legacy accounts during the transition period.
            </p>
          </div>
          <Form method="post" encType="application/x-www-form-urlencoded" className="space-y-4">
            <div>
              <label
                htmlFor="email"
                className="block text-sm font-medium text-gray-700 dark:text-gray-300"
              >
                Email
              </label>
              <input
                id="email"
                name="email"
                type="email"
                required
                className="mt-1 block w-full rounded-md border-gray-300 p-2 border shadow-sm focus:border-indigo-500 focus:ring-indigo-500 dark:border-gray-600 dark:bg-gray-800 dark:text-gray-100"
                placeholder="you@example.com"
              />
            </div>
            <div>
              <label
                htmlFor="password"
                className="block text-sm font-medium text-gray-700 dark:text-gray-300"
              >
                Password
              </label>
              <input
                id="password"
                name="password"
                type="password"
                required
                className="mt-1 block w-full rounded-md border-gray-300 p-2 border shadow-sm focus:border-indigo-500 focus:ring-indigo-500 dark:border-gray-600 dark:bg-gray-800 dark:text-gray-100"
              />
            </div>
            <button
              type="submit"
              className="w-full inline-flex justify-center rounded-md border border-transparent bg-indigo-600 px-4 py-2 text-sm font-medium text-white shadow-sm hover:bg-indigo-700 focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:ring-offset-2 dark:focus:ring-offset-gray-900"
            >
              Sign in
            </button>
          </Form>
        </>
      )}
    </div>
  );
}
