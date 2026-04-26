import { useMemo, useState } from "react";

// ---------------------------------------------------------------------------
// Spec 108 Phase 4 (UI follow-up). Reusable detail viewer for factory
// adapter manifests, contract schemas, and process definitions. Replaces
// the previous "JSON.stringify the whole thing" right-side panel: bodies
// that arrive as YAML / Markdown strings now render with real line breaks,
// JSON bodies are pretty-printed, and the raw and metadata views stay one
// tab away.
// ---------------------------------------------------------------------------

export type FactoryArtifact = {
  id?: string;
  name?: string;
  path?: string;
  body?: unknown;
  sourceSha?: string;
  sha?: string;
  version?: string;
  syncedAt?: string;
  [key: string]: unknown;
};

export type ArtifactBodyKind = "markdown" | "yaml" | "json" | "text";

type EnvelopeBody = { path: string; body: string };

function isEnvelopeBody(value: unknown): value is EnvelopeBody {
  return (
    value !== null &&
    typeof value === "object" &&
    !Array.isArray(value) &&
    typeof (value as { path?: unknown }).path === "string" &&
    typeof (value as { body?: unknown }).body === "string"
  );
}

// The translator wraps raw schema files as `{ path, body }` envelopes
// (api/factory/translator.ts). When that's what the API returned we hoist
// path + body to the top level so the viewer treats the inner string as
// the document instead of pretty-printing the wrapper.
export function unwrapArtifactEnvelope(artifact: FactoryArtifact): FactoryArtifact {
  if (isEnvelopeBody(artifact.body)) {
    return {
      ...artifact,
      path: artifact.path ?? artifact.body.path,
      body: artifact.body.body,
    };
  }
  return artifact;
}

export function detectArtifactBodyKind(artifact: FactoryArtifact): ArtifactBodyKind {
  const path = String(artifact.path ?? "").toLowerCase();

  if (path.endsWith(".schema.yaml") || path.endsWith(".schema.yml")) return "yaml";
  if (path.endsWith(".yaml") || path.endsWith(".yml")) return "yaml";
  if (path.endsWith(".json")) return "json";
  if (path.endsWith(".md")) return "markdown";

  // Object / array bodies come from jsonb columns (factory adapters &
  // processes) — render them as JSON.
  if (artifact.body != null && typeof artifact.body !== "string") return "json";

  const body = typeof artifact.body === "string" ? artifact.body : "";
  if (body.startsWith("---\n")) return "markdown";
  const trimmed = body.trimStart();
  if (trimmed.startsWith("{") || trimmed.startsWith("[")) return "json";
  return "text";
}

export function getArtifactDisplayBody(artifact: FactoryArtifact): string {
  const body = artifact.body;
  if (typeof body === "string") return body;
  if (body == null) return "";
  return JSON.stringify(body, null, 2);
}

export function getArtifactMetadata(
  artifact: FactoryArtifact
): Record<string, unknown> {
  const { body: _body, ...metadata } = artifact;
  void _body;
  return metadata;
}

function prettyJson(body: string): string {
  try {
    return JSON.stringify(JSON.parse(body), null, 2);
  } catch {
    return body;
  }
}

function formatLabel(kind: ArtifactBodyKind): string {
  switch (kind) {
    case "yaml":
      return "YAML";
    case "json":
      return "JSON";
    case "markdown":
      return "Markdown";
    case "text":
      return "Text";
  }
}

type Tab = "preview" | "source" | "metadata";

type Props = {
  artifact: FactoryArtifact;
  label?: string;
};

export function ArtifactBodyViewer({ artifact: rawArtifact, label }: Props) {
  const artifact = useMemo(() => unwrapArtifactEnvelope(rawArtifact), [rawArtifact]);
  const kind = useMemo(() => detectArtifactBodyKind(artifact), [artifact]);
  const body = useMemo(() => getArtifactDisplayBody(artifact), [artifact]);
  const metadata = useMemo(() => getArtifactMetadata(artifact), [artifact]);

  const [tab, setTab] = useState<Tab>("preview");
  const [wrap, setWrap] = useState(true);
  const [copied, setCopied] = useState(false);

  const isSchemaPath = typeof artifact.path === "string" && artifact.path.includes(".schema.");
  const headerName = artifact.name ?? label ?? "Artifact";

  const onCopy = async () => {
    if (!body) return;
    try {
      await navigator.clipboard.writeText(body);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      // Clipboard blocked — leave the body visible for manual copy.
    }
  };

  return (
    <div className="artifact-detail flex min-h-0 flex-col rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-900 overflow-hidden">
      <div className="artifact-detail__header sticky top-0 z-[1] border-b border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-900 px-4 py-3">
        <div className="flex flex-wrap items-baseline justify-between gap-x-3 gap-y-1">
          <div className="flex min-w-0 items-baseline gap-2">
            {label ? (
              <span className="text-[11px] uppercase tracking-wider font-medium text-gray-500 dark:text-gray-400 shrink-0">
                {label}
              </span>
            ) : null}
            <h3 className="font-mono text-base font-semibold text-gray-900 dark:text-gray-100 break-all">
              {headerName}
            </h3>
            {isSchemaPath ? (
              <span className="shrink-0 rounded bg-indigo-50 dark:bg-indigo-900/30 px-1.5 py-0.5 text-[10px] font-medium uppercase tracking-wider text-indigo-700 dark:text-indigo-300">
                schema
              </span>
            ) : null}
          </div>
          {typeof artifact.version === "string" ? (
            <span className="text-xs text-gray-400 dark:text-gray-500 shrink-0">
              v{artifact.version.slice(0, 12)}
            </span>
          ) : null}
        </div>
        <dl className="mt-2 grid grid-cols-1 sm:grid-cols-2 gap-x-6 gap-y-1 text-xs">
          {typeof artifact.path === "string" ? (
            <ProvRow label="path" value={artifact.path} mono />
          ) : null}
          <ProvRow label="format" value={formatLabel(kind)} />
          {typeof artifact.sourceSha === "string" ? (
            <ProvRow label="source sha" value={artifact.sourceSha} mono />
          ) : null}
          {typeof artifact.syncedAt === "string" ? (
            <ProvRow
              label="synced at"
              value={new Date(artifact.syncedAt).toLocaleString()}
            />
          ) : null}
        </dl>
        <div className="mt-3 flex flex-wrap items-center gap-1 border-b border-gray-200 dark:border-gray-700 -mb-3">
          <TabButton active={tab === "preview"} onClick={() => setTab("preview")}>
            Preview
          </TabButton>
          <TabButton active={tab === "source"} onClick={() => setTab("source")}>
            Source
          </TabButton>
          <TabButton active={tab === "metadata"} onClick={() => setTab("metadata")}>
            Metadata
          </TabButton>
          <div className="ml-auto flex items-center gap-3 pb-2 text-xs text-gray-500 dark:text-gray-400">
            {tab === "source" ? (
              <label className="flex items-center gap-1 cursor-pointer">
                <input
                  type="checkbox"
                  className="accent-indigo-600"
                  checked={wrap}
                  onChange={(e) => setWrap(e.target.checked)}
                />
                Wrap lines
              </label>
            ) : null}
            <button
              type="button"
              onClick={onCopy}
              disabled={!body}
              className="rounded border border-gray-200 dark:border-gray-700 px-2 py-0.5 text-[11px] font-medium hover:bg-gray-50 dark:hover:bg-gray-800 disabled:opacity-50"
            >
              {copied ? "Copied" : "Copy body"}
            </button>
          </div>
        </div>
      </div>

      <div className="artifact-detail__body min-h-0 overflow-auto px-4 py-3">
        {tab === "preview" ? (
          <PreviewBody body={body} kind={kind} />
        ) : tab === "source" ? (
          <SourceBody body={body} wrap={wrap} />
        ) : (
          <MetadataBody metadata={metadata} />
        )}
      </div>
    </div>
  );
}

function PreviewBody({ body, kind }: { body: string; kind: ArtifactBodyKind }) {
  if (!body) {
    return <EmptyBody />;
  }

  if (kind === "json") {
    return (
      <pre className="artifact-body artifact-body--json whitespace-pre-wrap break-words font-mono text-[12px] leading-relaxed text-gray-800 dark:text-gray-200">
        <code>{prettyJson(body)}</code>
      </pre>
    );
  }

  if (kind === "yaml") {
    return (
      <pre className="artifact-body artifact-body--yaml whitespace-pre-wrap break-words font-mono text-[12px] leading-relaxed text-gray-800 dark:text-gray-200">
        <code>{body}</code>
      </pre>
    );
  }

  if (kind === "markdown") {
    // No markdown renderer is wired up in stagecraft yet — preserving real
    // newlines + readable wrapping is the minimum the spec asks for.
    return (
      <pre className="artifact-body artifact-body--markdown whitespace-pre-wrap break-words font-sans text-sm leading-relaxed text-gray-800 dark:text-gray-200">
        {body}
      </pre>
    );
  }

  return (
    <pre className="artifact-body artifact-body--text whitespace-pre-wrap break-words font-sans text-sm leading-relaxed text-gray-800 dark:text-gray-200">
      {body}
    </pre>
  );
}

function SourceBody({ body, wrap }: { body: string; wrap: boolean }) {
  if (!body) {
    return <EmptyBody />;
  }
  return (
    <pre
      className={`artifact-source font-mono text-[12px] leading-relaxed text-gray-800 dark:text-gray-200 ${
        wrap ? "whitespace-pre-wrap break-words" : "whitespace-pre overflow-x-auto"
      }`}
    >
      <code>{body}</code>
    </pre>
  );
}

function MetadataBody({ metadata }: { metadata: Record<string, unknown> }) {
  const json = JSON.stringify(metadata, null, 2);
  return (
    <pre className="artifact-metadata whitespace-pre-wrap break-words font-mono text-[12px] leading-relaxed text-gray-800 dark:text-gray-200">
      <code>{json}</code>
    </pre>
  );
}

function EmptyBody() {
  return (
    <div className="text-sm text-gray-500 dark:text-gray-400 italic">
      No body content.
    </div>
  );
}

function TabButton({
  active,
  onClick,
  children,
}: {
  active: boolean;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-pressed={active}
      className={`px-3 py-1.5 text-xs font-medium border-b-2 -mb-px transition-colors ${
        active
          ? "border-indigo-600 text-indigo-700 dark:text-indigo-300"
          : "border-transparent text-gray-500 dark:text-gray-400 hover:text-gray-800 dark:hover:text-gray-200"
      }`}
    >
      {children}
    </button>
  );
}

function ProvRow({
  label,
  value,
  mono,
}: {
  label: string;
  value: string;
  mono?: boolean;
}) {
  return (
    <div className="flex gap-2 min-w-0">
      <dt className="text-gray-500 dark:text-gray-400 shrink-0">{label}</dt>
      <dd
        className={`text-gray-900 dark:text-gray-200 truncate ${
          mono ? "font-mono" : ""
        }`}
        title={value}
      >
        {value}
      </dd>
    </div>
  );
}
