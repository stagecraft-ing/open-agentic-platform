import { Topic } from "encore.dev/pubsub";

// ---------------------------------------------------------------------------
// Factory Pipeline Event Topic
// ---------------------------------------------------------------------------

export interface FactoryPipelineEvent {
  project_id: string;
  pipeline_id: string;
  event_type:
    | "pipeline_initialized"
    | "stage_confirmed"
    | "stage_rejected"
    | "pipeline_completed"
    | "pipeline_failed"
    | "deployment_triggered";
  stage_id?: string;
  actor?: string;
  details?: Record<string, unknown>;
}

export const FactoryEventTopic = new Topic<FactoryPipelineEvent>(
  "factory-event",
  {
    deliveryGuarantee: "at-least-once",
  }
);
