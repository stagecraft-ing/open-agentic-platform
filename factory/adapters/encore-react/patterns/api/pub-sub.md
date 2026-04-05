# Pub/Sub Pattern

Encore pub/sub uses `Topic` for publishing and `Subscription` for consuming.
Topics live in the producing service. Subscriptions live in the consuming service.

## Convention

- `new Topic<EventType>()` in the producing service
- `new Subscription(topic, name, { handler })` in the consuming service
- Event type exported as an interface alongside the topic
- Handler is an async function receiving the event
- `deliveryGuarantee: "at-least-once"` is the standard setting

## Template

### Topic (in the producing service)

```ts
import { Topic } from "encore.dev/pubsub";

// {EventDescription}
export interface {Event} {
  {field}: {type};
}

// '{topicName}' -- {topicDescription}
export const {TopicName} = new Topic<{Event}>("{topic.name}", {
  deliveryGuarantee: "at-least-once",
});
```

### Publishing (inside an endpoint or helper)

```ts
await {TopicName}.publish({eventData});
```

### Subscription (in the consuming service)

```ts
import { Subscription } from "encore.dev/pubsub";
import { {TopicName} } from "../{producerService}/{file}";

const _ = new Subscription({TopicName}, "{subscriptionName}", {
  handler: async (event) => {
    {handlerBody}
  },
});
```

## Example

Three-stage event chain from the fullstack-app:

**Stage 1 -- site publishes `site.added`** (`api/site/site.ts`):

```ts
export interface Site { id: number; url: string; }

export const SiteAddedTopic = new Topic<Site>("site.added", {
  deliveryGuarantee: "at-least-once",
});

export const add = api(
  { expose: true, method: "POST", path: "/site" },
  async (params: AddParams): Promise<Site> => {
    const site = (await Sites().insert({ url: params.url }, "*"))[0];
    await SiteAddedTopic.publish(site);
    return site;
  },
);
```

**Stage 2 -- monitor subscribes + publishes transition** (`api/monitor/check.ts`):

```ts
import { Subscription, Topic } from "encore.dev/pubsub";
import { Site, SiteAddedTopic } from "../site/site";

export interface TransitionEvent { site: Site; up: boolean; }

export const TransitionTopic = new Topic<TransitionEvent>(
  "uptime-transition", { deliveryGuarantee: "at-least-once" },
);

const _ = new Subscription(SiteAddedTopic, "check-site", {
  handler: doCheck,  // runs when a site is added
});

async function doCheck(site: Site): Promise<{ up: boolean }> {
  const { up } = await ping({ url: site.url });
  if (up !== await getPreviousMeasurement(site.id)) {
    await TransitionTopic.publish({ site, up });
  }
  return { up };
}
```

**Stage 3 -- slack subscribes to transition** (`api/slack/slack.ts`):

```ts
import { Subscription } from "encore.dev/pubsub";
import { TransitionTopic } from "../monitor/check";

const _ = new Subscription(TransitionTopic, "slack-notification", {
  handler: async (event) => {
    const text = `*${event.site.url} is ${event.up ? "back up." : "down!"}*`;
    await notify({ text });
  },
});
```

## Rules

1. Topic names use dot or dash separators (`"site.added"`, `"uptime-transition"`).
2. Export the topic and its event interface so other services can import them.
3. Subscription names must be unique across the entire app.
4. Assign subscriptions to `const _` at module scope -- they activate on import.
5. Handlers must be idempotent: `at-least-once` means possible redelivery.
6. Subscriptions belong in the consuming service, not the topic's owner.
