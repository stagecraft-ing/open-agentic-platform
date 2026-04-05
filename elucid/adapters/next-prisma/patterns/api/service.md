# Service Pattern

Services encapsulate business logic and database access via Prisma Client.
Route handlers and Server Actions delegate to services — they never call Prisma directly.

## Convention

- One file per resource: `src/lib/services/{resource}.service.ts`
- Exported object with async methods
- All database access through the shared Prisma Client singleton
- Audit trail on every mutation (create, update, delete)

## Template

```ts
import { prisma } from "@/lib/db";
import type { User } from "next-auth";

export const {entity}Service = {
  async findAll() {
    return prisma.{entity}.findMany({
      orderBy: { createdAt: "desc" },
    });
  },

  async findById(id: string) {
    return prisma.{entity}.findUnique({ where: { id } });
  },

  async create(data: Create{Entity}Input, user: User) {
    const item = await prisma.{entity}.create({ data });

    await prisma.auditEntry.create({
      data: {
        userId: user.id,
        actionCode: "create_{entity_snake}",
        entityType: "{Entity}",
        entityId: item.id,
      },
    });

    return item;
  },

  async update(id: string, data: Update{Entity}Input, user: User) {
    const item = await prisma.{entity}.update({
      where: { id },
      data,
    });

    await prisma.auditEntry.create({
      data: {
        userId: user.id,
        actionCode: "update_{entity_snake}",
        entityType: "{Entity}",
        entityId: id,
      },
    });

    return item;
  },

  async delete(id: string, user: User) {
    await prisma.{entity}.delete({ where: { id } });

    await prisma.auditEntry.create({
      data: {
        userId: user.id,
        actionCode: "delete_{entity_snake}",
        entityType: "{Entity}",
        entityId: id,
      },
    });
  },
};
```

## Prisma Client Singleton

```ts
// src/lib/db.ts
import { PrismaClient } from "@prisma/client";

const globalForPrisma = globalThis as unknown as { prisma: PrismaClient };

export const prisma = globalForPrisma.prisma ?? new PrismaClient();

if (process.env.NODE_ENV !== "production") globalForPrisma.prisma = prisma;
```

## Example

From `src/lib/services/sites.service.ts`:

```ts
import { prisma } from "@/lib/db";

export const siteService = {
  async findAll() {
    return prisma.site.findMany({ orderBy: { id: "asc" } });
  },

  async create(data: { url: string }) {
    return prisma.site.create({ data: { url: data.url } });
  },

  async delete(id: number) {
    await prisma.site.delete({ where: { id } });
  },
};
```

## Rules

1. Import `prisma` from `@/lib/db` — never create new `PrismaClient()` instances.
2. Every mutation writes an audit entry (if audit is enabled in Build Spec).
3. Use `findUnique` for single-record lookups, `findMany` for lists.
4. Use Prisma's `include` and `select` for relations — no N+1 queries.
5. Services throw on errors — callers (handlers/actions) catch and format HTTP responses.
6. Never import services in Client Components — only in Server Components, Route Handlers, and Server Actions.
