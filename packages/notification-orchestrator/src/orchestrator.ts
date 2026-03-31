import type {
  NotificationEvent,
  NotificationKind,
  Severity,
  ChannelAdapter,
  NotifyResult,
} from "./types.js";

/**
 * Options for creating a notification event via {@link NotificationOrchestrator.notify}.
 * All fields from {@link NotificationEvent} except `id` and `timestamp`,
 * which are generated automatically.
 */
export interface NotifyOptions {
  provider: string;
  sessionId: string;
  kind: NotificationKind;
  severity: Severity;
  dedupeKey: string;
  title: string;
  body: string;
  metadata?: Record<string, unknown>;
}

/**
 * Core notification orchestrator (Phase 1).
 *
 * Accepts typed notification events via `notify()`, builds a full
 * {@link NotificationEvent}, and dispatches to all registered
 * {@link ChannelAdapter}s that report themselves as available.
 *
 * Phase 1 dispatches to all adapters unconditionally. Deduplication (Phase 2)
 * and preference-gated routing (Phase 3) will be inserted into the dispatch
 * pipeline in subsequent phases.
 */
export class NotificationOrchestrator {
  private adapters: Map<string, ChannelAdapter> = new Map();

  /**
   * Register a channel adapter. Replaces any existing adapter with the
   * same `channelId`.
   */
  registerAdapter(adapter: ChannelAdapter): void {
    this.adapters.set(adapter.channelId, adapter);
  }

  /**
   * Remove a previously registered adapter by channel id.
   * Returns `true` if the adapter was found and removed.
   */
  unregisterAdapter(channelId: string): boolean {
    return this.adapters.delete(channelId);
  }

  /**
   * Get all registered adapter channel ids.
   */
  getAdapterIds(): string[] {
    return [...this.adapters.keys()];
  }

  /**
   * Emit a notification event and dispatch to all available adapters (FR-001).
   *
   * Generates `id` (UUID) and `timestamp` (Date.now()) automatically.
   * Returns a {@link NotifyResult} describing delivery outcomes.
   */
  async notify(options: NotifyOptions): Promise<NotifyResult> {
    const event: NotificationEvent = {
      id: crypto.randomUUID(),
      provider: options.provider,
      sessionId: options.sessionId,
      kind: options.kind,
      severity: options.severity,
      dedupeKey: options.dedupeKey,
      title: options.title,
      body: options.body,
      timestamp: Date.now(),
      metadata: options.metadata ?? {},
    };

    return this.dispatch(event);
  }

  /**
   * Internal dispatch loop. Iterates all registered adapters, skips
   * unavailable ones, and collects results.
   */
  private async dispatch(event: NotificationEvent): Promise<NotifyResult> {
    const deliveredTo: string[] = [];
    const failures: Array<{ channelId: string; error: string }> = [];

    const availableAdapters = [...this.adapters.values()].filter((a) =>
      a.isAvailable()
    );

    if (availableAdapters.length === 0) {
      return {
        eventId: event.id,
        status: "suppressed",
        deliveredTo: [],
        failures: [],
      };
    }

    await Promise.all(
      availableAdapters.map(async (adapter) => {
        try {
          await adapter.deliver(event);
          deliveredTo.push(adapter.channelId);
        } catch (err) {
          failures.push({
            channelId: adapter.channelId,
            error: err instanceof Error ? err.message : String(err),
          });
        }
      })
    );

    let status: NotifyResult["status"];
    if (failures.length === 0) {
      status = "delivered";
    } else if (deliveredTo.length === 0) {
      status = "suppressed";
    } else {
      status = "partial";
    }

    return { eventId: event.id, status, deliveredTo, failures };
  }
}
