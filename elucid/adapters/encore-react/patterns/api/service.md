# Service Pattern

Every Encore service has a directory with an `encore.service.ts` file that registers
it. Infrastructure (databases, secrets) is declared as module-level constants inside
service files. Inter-service calls use the generated `~encore/clients` module.

## Convention

- One `encore.service.ts` per service directory -- the `new Service()` call defines
  the service boundary and its name
- `SQLDatabase` declared at module scope in the file that uses it
- Secrets via `secret()` from `encore.dev/config`
- Cross-service calls: import from `~encore/clients`, never direct file imports

## Template

### encore.service.ts

```ts
import { Service } from "encore.dev/service";

// {ServiceDescription}
export default new Service("{serviceName}");
```

### Database declaration (in the service's main .ts file)

```ts
import { SQLDatabase } from "encore.dev/storage/sqldb";

// The '{dbName}' database -- Encore auto-provisions, migrates, and connects.
export const {DbName}DB = new SQLDatabase("{dbName}", {
  migrations: "./migrations",
});
```

### Secrets (in the service's main .ts file)

```ts
import { secret } from "encore.dev/config";

const {secretName} = secret("{SecretName}");
```

### Inter-service call

```ts
import { {remoteService} } from "~encore/clients";

const result = await {remoteService}.{endpoint}({params});
```

## Example

Service registration -- `api/monitor/encore.service.ts`:

```ts
import { Service } from "encore.dev/service";

// The monitor service pings sites and stores the results in the database.
export default new Service("monitor");
```

Database + inter-service call -- `api/monitor/check.ts`:

```ts
import { SQLDatabase } from "encore.dev/storage/sqldb";
import { site } from "~encore/clients";

export const MonitorDB = new SQLDatabase("monitor", {
  migrations: "./migrations",
});

export const check = api(
  { expose: true, method: "POST", path: "/check/:siteID" },
  async (p: { siteID: number }): Promise<{ up: boolean }> => {
    const s = await site.get({ id: p.siteID });   // cross-service call
    return doCheck(s);
  },
);
```

ORM integration -- `api/db/drizzle.ts`:

```ts
import { SQLDatabase } from "encore.dev/storage/sqldb";
import { drizzle } from "drizzle-orm/node-postgres";

const AuthDB = new SQLDatabase("auth", {
  migrations: "./migrations",
});

export const db = drizzle(AuthDB.connectionString);
```

Secrets -- `api/slack/slack.ts`:

```ts
import { secret } from "encore.dev/config";

const webhookURL = secret("SlackWebhookURL");

// Use inside a handler:
const url = webhookURL();
```

## Rules

1. Exactly one `encore.service.ts` per service directory. The file must
   `export default new Service("{name}")`.
2. The service name string must match the directory name.
3. `SQLDatabase` is declared at module scope -- never inside a handler.
4. Pass `{ migrations: "./migrations" }` to point at the service's SQL migration
   folder. Encore runs migrations automatically on deploy.
5. Use `SQLDatabase.connectionString` when bridging to an ORM (Knex, Drizzle).
6. Use `SQLDatabase.exec` or `SQLDatabase.query` for raw tagged-template SQL.
7. Secrets: `secret("X")` returns a getter; call it as `secret("X")()` for the value.
8. Never import another service's files directly. Use `~encore/clients` for RPC.
9. Each service owns its database. No service reads another service's DB.
