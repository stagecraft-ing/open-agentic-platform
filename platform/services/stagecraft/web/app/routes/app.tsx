import { Outlet, useLoaderData } from "react-router";
import { Link } from "react-router";
import { requireUser } from "../lib/auth.server";

export async function loader({ request }: { request: Request }) {
  const user = await requireUser(request);
  return { user };
}

export default function AppLayout() {
  const { user } = useLoaderData() as { user: { name: string; email: string } };
  return (
    <div className="min-h-full container px-4 mx-auto my-8">
      <nav className="flex gap-4 mb-8 border-b border-gray-200 dark:border-gray-700 pb-4">
        <Link
          to="/app"
          className="text-gray-700 hover:text-gray-900 dark:text-gray-300 dark:hover:text-gray-100"
        >
          Dashboard
        </Link>
        <Link
          to="/app/settings"
          className="text-gray-700 hover:text-gray-900 dark:text-gray-300 dark:hover:text-gray-100"
        >
          Settings
        </Link>
      </nav>
      <main>
        <h2 className="text-xl font-semibold text-gray-900 dark:text-gray-100 mb-4">
          Welcome, {user.name}
        </h2>
        <Outlet />
      </main>
    </div>
  );
}
