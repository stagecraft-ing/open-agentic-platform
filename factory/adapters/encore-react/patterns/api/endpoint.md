# Endpoint Pattern

Encore API endpoints are exported `const` values created with `api()`. Each endpoint
is a self-contained handler -- no controller class, no separate route registration.

## Convention

- One file per domain concept (site.ts, auth.ts, admin.ts)
- Request/response types as `interface` or `type` above the endpoint
- `api()` receives an options object first, then an async handler function
- Handler contains the full business logic (DB queries, pub/sub, etc.)
- Errors: throw or return a rejection; Encore maps thrown errors to HTTP errors

## Template

```ts
import { api } from "encore.dev/api";

// {EntityDescription}
export interface {Entity} {
  {field}: {type};
}

// {ActionDescription}
export interface {Action}Params {
  {field}: {type};
}

export interface {Action}Response {
  {field}: {type};
}

// {EndpointComment}
export const {name} = api(
  { expose: {expose}, method: "{METHOD}", path: "{/path}", auth: {auth} },
  async ({params}: {Action}Params): Promise<{Action}Response> => {
    // direct service logic here -- no controller layer
    {body}
  },
);
```

## Example

From `api/site/site.ts` -- CRUD endpoints with inline DB access and pub/sub:

```ts
import { api } from "encore.dev/api";
import { Topic } from "encore.dev/pubsub";
import { SQLDatabase } from "encore.dev/storage/sqldb";
import knex from "knex";

export interface Site {
  id: number;
  url: string;
}

export const SiteAddedTopic = new Topic<Site>("site.added", {
  deliveryGuarantee: "at-least-once",
});

export interface AddParams {
  url: string;
}

export const add = api(
  { expose: true, method: "POST", path: "/site" },
  async (params: AddParams): Promise<Site> => {
    const site = (await Sites().insert({ url: params.url }, "*"))[0];
    await SiteAddedTopic.publish(site);
    return site;
  },
);

export const get = api(
  { expose: true, method: "GET", path: "/site/:id", auth: false },
  async ({ id }: { id: number }): Promise<Site> => {
    const site = await Sites().where("id", id).first();
    return site ?? Promise.reject(new Error("site not found"));
  },
);

export const del = api(
  { expose: true, method: "DELETE", path: "/site/:id" },
  async ({ id }: { id: number }): Promise<void> => {
    await Sites().where("id", id).delete();
  },
);

export interface ListResponse {
  sites: Site[];
}

export const list = api(
  { expose: true, method: "GET", path: "/site" },
  async (): Promise<ListResponse> => {
    const sites = await Sites().select();
    return { sites };
  },
);
```

Typed generic form (from `api/monitor/ping.ts`):

```ts
export const ping = api<PingParams, PingResponse>(
  { expose: true, path: "/ping/:url", method: "GET" },
  async ({ url }) => { /* handler body */ },
);
```

## Rules

1. Every endpoint is an `export const` -- never a class method.
2. Options object is always the first argument: `{ expose, method, path }`.
3. Path params use `:param` syntax and must appear in the request type.
4. `expose: true` makes it reachable from the frontend; omit or set `false` for internal.
5. `auth: false` opts out of the service-level auth middleware.
6. Return `Promise<void>` for side-effect-only endpoints (DELETE, etc.).
7. Inline the logic. Import shared helpers from sibling files within the service.
