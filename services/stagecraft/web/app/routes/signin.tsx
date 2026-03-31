import { Form, Link, redirect, useActionData } from "react-router";
import { authSignin } from "../lib/auth-api.server";
import { getFormValues } from "../lib/form-data.server";

export async function action({ request }: { request: Request }) {
  const data = await getFormValues(request);
  const email = String(data.email || "");
  const password = String(data.password || "");

  const res = await authSignin(request, email, password);
  if (!res.ok) return { error: res.error || "Sign-in failed" };

  return redirect("/app", {
    headers: { "Set-Cookie": res.setCookie },
  });
}

export default function Signin() {
  const data = useActionData() as { error?: string } | undefined;
  return (
    <div className="min-h-full container px-4 mx-auto my-16 max-w-sm">
      <h1 className="text-2xl font-bold text-gray-900 dark:text-gray-100">
        Sign in
      </h1>
      {data?.error ? (
        <p className="mt-2 text-sm text-red-600 dark:text-red-400">
          {data.error}
        </p>
      ) : null}
      <Form method="post" encType="application/x-www-form-urlencoded" className="mt-4 space-y-4">
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
      <p className="mt-4 text-sm text-gray-600 dark:text-gray-400">
        Don&apos;t have an account?{" "}
        <Link to="/signup" className="text-indigo-600 hover:text-indigo-500 dark:text-indigo-400">
          Create account
        </Link>
      </p>
    </div>
  );
}
