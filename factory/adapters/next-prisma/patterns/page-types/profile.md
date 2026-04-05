# Profile Page Pattern

User profile/settings page. Server Component fetches user data;
Client Component handles form edits.

## Template

```tsx
import { getServerSession } from "next-auth";
import { authOptions } from "@/lib/auth";
import { redirect } from "next/navigation";
import { prisma } from "@/lib/db";
import { ProfileForm } from "@/components/ProfileForm.client";
import { updateProfile } from "./actions";

export default async function ProfilePage() {
  const session = await getServerSession(authOptions);
  if (!session) redirect("/auth/signin");

  const user = await prisma.user.findUnique({
    where: { id: session.user.id },
  });

  if (!user) redirect("/auth/signin");

  return (
    <div className="max-w-2xl mx-auto space-y-6">
      <h1 className="text-2xl font-bold">Profile Settings</h1>

      <div className="border rounded-lg p-6 space-y-4">
        <h2 className="text-lg font-semibold">Account Information</h2>
        <ProfileForm user={user} action={updateProfile} />
      </div>

      <div className="border rounded-lg p-6">
        <h2 className="text-lg font-semibold">Sessions</h2>
        <p className="text-sm text-gray-500">
          Signed in as {user.email}
        </p>
      </div>
    </div>
  );
}
```

## Rules

1. Fetch current user via session — no user ID in the URL.
2. Server Action for profile updates.
3. Display account info, session details, and role.
4. Separate sections for different settings categories.
