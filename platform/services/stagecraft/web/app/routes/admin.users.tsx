import { Form, redirect, useLoaderData } from "react-router";
import { createEncoreClient } from "../lib/encore.server";
import { requireAdmin } from "../lib/auth.server";
import { getFormValues } from "../lib/form-data.server";

export async function loader({ request }: { request: Request }) {
  const admin = await requireAdmin(request);
  const client = createEncoreClient(request);
  const res = await client.admin.listUsers();
  return { admin, users: res.users };
}

export async function action({ request }: { request: Request }) {
  const admin = await requireAdmin(request);
  const data = await getFormValues(request);
  const userId = String(data.userId);
  const role = String(data.role) as "user" | "admin";

  const client = createEncoreClient(request);
  await client.admin.setRole({
    actorUserId: admin.userId,
    userId,
    role,
  });
  return redirect("/admin/users");
}

export default function AdminUsers() {
  const { users } = useLoaderData() as {
    users: Array<{
      id: string;
      email: string;
      name: string;
      role: string;
      disabled: boolean;
      createdAt: string;
    }>;
  };

  return (
    <div>
      <h3 className="text-lg font-medium text-gray-900 dark:text-gray-100 mb-4">
        Users
      </h3>
      <ul className="divide-y divide-gray-200 dark:divide-gray-700">
        {users.map((u) => (
          <li key={u.id} className="py-3 flex items-center justify-between gap-4">
            <span className="text-gray-700 dark:text-gray-300">
              {u.email} ({u.role})
            </span>
            <Form method="post" encType="application/x-www-form-urlencoded" className="flex items-center gap-2">
              <input type="hidden" name="userId" value={u.id} />
              <select
                name="role"
                defaultValue={u.role}
                className="rounded-md border-gray-300 p-1 text-sm dark:border-gray-600 dark:bg-gray-800 dark:text-gray-100"
              >
                <option value="user">user</option>
                <option value="admin">admin</option>
              </select>
              <button
                type="submit"
                className="rounded-md border border-transparent bg-indigo-600 px-3 py-1 text-sm font-medium text-white hover:bg-indigo-700"
              >
                Update
              </button>
            </Form>
          </li>
        ))}
      </ul>
    </div>
  );
}
