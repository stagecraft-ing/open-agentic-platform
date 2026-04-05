# Route Handler Pattern

Next.js Route Handlers are exported async functions named after HTTP methods. Each file
in `src/app/api/` becomes an API endpoint via the file system router.

## Convention

- One `route.ts` per resource directory; dynamic segments use `[id]/route.ts`
- Export named functions: `GET`, `POST`, `PUT`, `PATCH`, `DELETE`
- Auth checked via `getServerSession()` at the top of protected handlers
- Request validation with Zod before passing to service layer
- Return `NextResponse.json()` with explicit status codes

## Template

```ts
import { NextResponse } from "next/server";
import { getServerSession } from "next-auth";
import { authOptions } from "@/lib/auth";
import { {entity}Service } from "@/lib/services/{resource}.service";
import { Create{Entity}Schema } from "@/lib/types/{entity}";

export async function GET() {
  const session = await getServerSession(authOptions);
  if (!session) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  const items = await {entity}Service.findAll();
  return NextResponse.json({ {items}: items });
}

export async function POST(request: Request) {
  const session = await getServerSession(authOptions);
  if (!session) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  const body = await request.json();
  const parsed = Create{Entity}Schema.safeParse(body);
  if (!parsed.success) {
    return NextResponse.json({ error: parsed.error.flatten() }, { status: 400 });
  }

  const item = await {entity}Service.create(parsed.data, session.user);
  return NextResponse.json(item, { status: 201 });
}
```

### Dynamic route (`[id]/route.ts`)

```ts
import { NextResponse } from "next/server";
import { getServerSession } from "next-auth";
import { authOptions } from "@/lib/auth";
import { {entity}Service } from "@/lib/services/{resource}.service";

type Params = { params: Promise<{ id: string }> };

export async function GET(_request: Request, { params }: Params) {
  const session = await getServerSession(authOptions);
  if (!session) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  const { id } = await params;
  const item = await {entity}Service.findById(id);
  if (!item) {
    return NextResponse.json({ error: "Not found" }, { status: 404 });
  }
  return NextResponse.json(item);
}

export async function DELETE(_request: Request, { params }: Params) {
  const session = await getServerSession(authOptions);
  if (!session) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  const { id } = await params;
  await {entity}Service.delete(id, session.user);
  return new NextResponse(null, { status: 204 });
}
```

## Example

From `src/app/api/sites/route.ts`:

```ts
import { NextResponse } from "next/server";
import { getServerSession } from "next-auth";
import { authOptions } from "@/lib/auth";
import { siteService } from "@/lib/services/sites.service";
import { CreateSiteSchema } from "@/lib/types/site";

export async function GET() {
  const session = await getServerSession(authOptions);
  if (!session) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }
  const sites = await siteService.findAll();
  return NextResponse.json({ sites });
}

export async function POST(request: Request) {
  const session = await getServerSession(authOptions);
  if (!session) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }
  const body = await request.json();
  const parsed = CreateSiteSchema.safeParse(body);
  if (!parsed.success) {
    return NextResponse.json({ error: parsed.error.flatten() }, { status: 400 });
  }
  const site = await siteService.create(parsed.data);
  return NextResponse.json(site, { status: 201 });
}
```

## Rules

1. Every handler is an exported named function matching an HTTP method.
2. `getServerSession(authOptions)` for auth — never decode tokens manually.
3. Validate bodies with Zod `.safeParse()` — return 400 on failure.
4. Delegate business logic to service layer — handlers are thin HTTP adapters.
5. Use `NextResponse.json()` for JSON responses, `new NextResponse(null, { status })` for empty.
6. Dynamic segments use `[id]` directory naming; access via `params` arg.
7. Never import Prisma Client directly in route handlers — use the service layer.
