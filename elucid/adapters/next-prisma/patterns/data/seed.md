# Seed Pattern

Database seeding populates reference data and development fixtures.
Run with `npx prisma db seed`.

## Convention

- Single file: `prisma/seed.ts`
- Configured in `package.json`: `"prisma": { "seed": "npx tsx prisma/seed.ts" }`
- Idempotent — use `upsert` to avoid duplicate key errors
- Seed reference data (roles, statuses) and dev fixtures

## Template

```ts
import { PrismaClient } from "@prisma/client";

const prisma = new PrismaClient();

async function main() {
  // Reference data (always seeded)
  await prisma.{entity}.upsert({
    where: { {uniqueField}: "{value}" },
    update: {},
    create: {
      {field}: "{value}",
      {field2}: "{value2}",
    },
  });

  // Development fixtures (only in dev)
  if (process.env.NODE_ENV !== "production") {
    await prisma.user.upsert({
      where: { email: "admin@example.com" },
      update: {},
      create: {
        email: "admin@example.com",
        name: "Admin User",
      },
    });
  }

  console.log("Seed complete");
}

main()
  .catch((e) => {
    console.error(e);
    process.exit(1);
  })
  .finally(async () => {
    await prisma.$disconnect();
  });
```

## Example

```ts
import { PrismaClient } from "@prisma/client";
const prisma = new PrismaClient();

async function main() {
  const site = await prisma.site.upsert({
    where: { url: "https://example.com" },
    update: {},
    create: { url: "https://example.com" },
  });

  await prisma.check.create({
    data: { siteId: site.id, up: true },
  });
}

main()
  .catch(console.error)
  .finally(() => prisma.$disconnect());
```

## Rules

1. Use `upsert` for reference data — seeds must be idempotent.
2. Guard dev-only fixtures with `NODE_ENV !== "production"`.
3. Always `$disconnect()` in the `finally` block.
4. Seed in dependency order — parent entities before children.
5. Configure seed command in `package.json` prisma section.
