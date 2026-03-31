import { useLoaderData } from "react-router";
import { createEncoreClient } from "../lib/encore.server";
import { requireAdmin } from "../lib/auth.server";

export async function loader({ request }: { request: Request }) {
  await requireAdmin(request);
  const client = createEncoreClient(request);
  const res = await client.admin.listAudit();
  return { events: res.events };
}

export default function AdminAudit() {
  const { events } = useLoaderData() as {
    events: Array<{
      id: string;
      actorUserId: string;
      action: string;
      targetType: string;
      targetId: string;
      metadata: Record<string, unknown>;
      createdAt: string;
    }>;
  };

  return (
    <div>
      <h3 className="text-lg font-medium text-gray-900 dark:text-gray-100 mb-4">
        Audit
      </h3>
      <ul className="divide-y divide-gray-200 dark:divide-gray-700">
        {events.map((e) => (
          <li key={e.id} className="py-2 text-sm text-gray-700 dark:text-gray-300">
            {e.createdAt}: {e.action} target={e.targetType}:{e.targetId}
          </li>
        ))}
      </ul>
    </div>
  );
}
