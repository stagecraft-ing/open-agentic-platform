// Spec: specs/076-factory-desktop-panel/spec.md
// Structured rendering of a Build Spec YAML for the stage 5 approval gate (FR-005).

import React, { useState } from 'react';
import { ChevronDown, ChevronRight } from 'lucide-react';
import { cn } from '@/lib/utils';

// ── Props ─────────────────────────────────────────────────────────────────────

interface BuildSpecStructuredViewProps {
  buildSpec: any; // Parsed Build Spec object (from YAML)
}

// ── Shared primitives ─────────────────────────────────────────────────────────

interface CollapsibleSectionProps {
  title: string;
  badge?: string;
  defaultOpen?: boolean;
  children: React.ReactNode;
}

const CollapsibleSection: React.FC<CollapsibleSectionProps> = ({
  title,
  badge,
  defaultOpen = true,
  children,
}) => {
  const [open, setOpen] = useState(defaultOpen);

  return (
    <div className="bg-card border border-border rounded-lg overflow-hidden">
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        className="w-full flex items-center gap-2 px-4 py-3 text-left hover:bg-accent/40 transition-colors"
      >
        {open ? (
          <ChevronDown className="h-4 w-4 shrink-0 text-muted-foreground" />
        ) : (
          <ChevronRight className="h-4 w-4 shrink-0 text-muted-foreground" />
        )}
        <span className="text-sm font-medium text-foreground flex-1">{title}</span>
        {badge && (
          <span className="bg-muted text-muted-foreground rounded-full px-2 py-0.5 text-xs">
            {badge}
          </span>
        )}
      </button>
      {open && (
        <div className="px-4 pb-4 border-t border-border">{children}</div>
      )}
    </div>
  );
};

// ── Method badge ──────────────────────────────────────────────────────────────

const METHOD_COLORS: Record<string, string> = {
  GET: 'bg-emerald-900/40 text-emerald-400 border border-emerald-800',
  POST: 'bg-blue-900/40 text-blue-400 border border-blue-800',
  PUT: 'bg-amber-900/40 text-amber-400 border border-amber-800',
  PATCH: 'bg-amber-900/40 text-amber-400 border border-amber-800',
  DELETE: 'bg-red-900/40 text-red-400 border border-red-800',
};

const MethodBadge: React.FC<{ method: string }> = ({ method }) => {
  const upper = method.toUpperCase();
  const colorClass =
    METHOD_COLORS[upper] ?? 'bg-muted text-muted-foreground border border-border';
  return (
    <span
      className={cn(
        'inline-flex items-center rounded px-1.5 py-0.5 text-xs font-mono font-semibold',
        colorClass,
      )}
    >
      {upper}
    </span>
  );
};

// ── ConstraintBadge ───────────────────────────────────────────────────────────

const ConstraintBadge: React.FC<{ label: string }> = ({ label }) => (
  <span className="bg-muted text-muted-foreground rounded-full px-2 py-0.5 text-xs">
    {label}
  </span>
);

// ── Table helpers ─────────────────────────────────────────────────────────────

const Th: React.FC<{ children: React.ReactNode; className?: string }> = ({
  children,
  className,
}) => (
  <th
    className={cn(
      'text-left text-xs text-muted-foreground uppercase tracking-wide font-medium py-2 px-3',
      className,
    )}
  >
    {children}
  </th>
);

const Td: React.FC<{ children: React.ReactNode; className?: string }> = ({
  children,
  className,
}) => (
  <td className={cn('py-2 px-3 text-sm text-foreground align-top', className)}>
    {children}
  </td>
);

// ── 1. ProjectHeader ──────────────────────────────────────────────────────────

const ProjectHeader: React.FC<{ project: any }> = ({ project }) => (
  <div className="pt-3 space-y-1">
    <div className="flex items-center gap-3">
      <h2 className="text-base font-semibold text-foreground">
        {project.name ?? '—'}
      </h2>
      {project.variant && (
        <ConstraintBadge label={project.variant} />
      )}
    </div>
    {project.description && (
      <p className="text-sm text-muted-foreground">{project.description}</p>
    )}
  </div>
);

// ── 2. AuthTable ──────────────────────────────────────────────────────────────

const AuthTable: React.FC<{ audiences: any[] }> = ({ audiences }) => (
  <div className="pt-3 overflow-x-auto">
    <table className="w-full border-collapse">
      <thead>
        <tr className="border-b border-border">
          <Th>Name</Th>
          <Th>Method</Th>
          <Th>Provider</Th>
          <Th>Roles</Th>
        </tr>
      </thead>
      <tbody>
        {audiences.map((a, i) => (
          <tr key={i} className="border-b border-border last:border-0">
            <Td className="font-mono">{a.name ?? '—'}</Td>
            <Td>{a.method ?? '—'}</Td>
            <Td>{a.provider ?? '—'}</Td>
            <Td>
              <div className="flex flex-wrap gap-1">
                {Array.isArray(a.roles)
                  ? a.roles.map((r: string, ri: number) => (
                      <ConstraintBadge key={ri} label={r} />
                    ))
                  : '—'}
              </div>
            </Td>
          </tr>
        ))}
      </tbody>
    </table>
  </div>
);

// ── 3. EntityCards ────────────────────────────────────────────────────────────

const EntityCard: React.FC<{ entity: any }> = ({ entity }) => (
  <div className="bg-muted/30 border border-border rounded-lg p-3 space-y-2">
    <div className="text-sm font-medium text-foreground font-mono">
      {entity.name ?? '—'}
    </div>
    {Array.isArray(entity.fields) && entity.fields.length > 0 && (
      <table className="w-full border-collapse">
        <thead>
          <tr className="border-b border-border">
            <Th>Field</Th>
            <Th>Type</Th>
            <Th>Constraints</Th>
          </tr>
        </thead>
        <tbody>
          {entity.fields.map((field: any, fi: number) => (
            <tr key={fi} className="border-b border-border last:border-0">
              <Td className="font-mono">{field.name ?? '—'}</Td>
              <Td className="text-muted-foreground">{field.type ?? '—'}</Td>
              <Td>
                <div className="flex flex-wrap gap-1">
                  {Array.isArray(field.constraints)
                    ? field.constraints.map((c: string, ci: number) => (
                        <ConstraintBadge key={ci} label={c} />
                      ))
                    : null}
                </div>
              </Td>
            </tr>
          ))}
        </tbody>
      </table>
    )}
  </div>
);

const EntityCards: React.FC<{ entities: any[] }> = ({ entities }) => (
  <div className="pt-3 space-y-3">
    {entities.map((entity, i) => (
      <EntityCard key={i} entity={entity} />
    ))}
  </div>
);

// ── 4. ApiOperationTree ───────────────────────────────────────────────────────

const ApiResourceRow: React.FC<{ resource: any }> = ({ resource }) => {
  const [open, setOpen] = useState(true);
  const operations: any[] = Array.isArray(resource.operations)
    ? resource.operations
    : [];

  return (
    <div className="border border-border rounded-lg overflow-hidden">
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        className="w-full flex items-center gap-2 px-3 py-2 text-left bg-muted/20 hover:bg-accent/30 transition-colors"
      >
        {open ? (
          <ChevronDown className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
        ) : (
          <ChevronRight className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
        )}
        <span className="text-sm font-medium text-foreground font-mono flex-1">
          {resource.name ?? resource.path ?? '—'}
        </span>
        <span className="text-xs text-muted-foreground">
          {operations.length} op{operations.length !== 1 ? 's' : ''}
        </span>
      </button>
      {open && operations.length > 0 && (
        <div className="divide-y divide-border">
          {operations.map((op, oi) => (
            <div key={oi} className="flex items-start gap-3 px-4 py-2">
              <MethodBadge method={op.method ?? 'GET'} />
              <div className="flex-1 min-w-0">
                <span className="text-xs font-mono text-muted-foreground truncate block">
                  {op.path ?? ''}
                </span>
                {op.summary && (
                  <span className="text-xs text-foreground">{op.summary}</span>
                )}
              </div>
              {Array.isArray(op.audiences) && op.audiences.length > 0 && (
                <div className="flex flex-wrap gap-1 shrink-0">
                  {op.audiences.map((aud: string, ai: number) => (
                    <ConstraintBadge key={ai} label={aud} />
                  ))}
                </div>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
};

const ApiOperationTree: React.FC<{ resources: any[] }> = ({ resources }) => (
  <div className="pt-3 space-y-2">
    {resources.map((resource, i) => (
      <ApiResourceRow key={i} resource={resource} />
    ))}
  </div>
);

// ── 5. PageCards ──────────────────────────────────────────────────────────────

const PAGE_TYPE_COLORS: Record<string, string> = {
  list: 'bg-blue-900/40 text-blue-300 border border-blue-800',
  detail: 'bg-violet-900/40 text-violet-300 border border-violet-800',
  form: 'bg-amber-900/40 text-amber-300 border border-amber-800',
  dashboard: 'bg-emerald-900/40 text-emerald-300 border border-emerald-800',
};

const PageCard: React.FC<{ page: any }> = ({ page }) => {
  const typeColor =
    PAGE_TYPE_COLORS[page.page_type ?? ''] ??
    'bg-muted text-muted-foreground border border-border';

  return (
    <div className="bg-muted/30 border border-border rounded-lg p-3 space-y-2">
      <div className="flex items-center gap-2">
        <span className="text-sm font-medium text-foreground">
          {page.title ?? page.name ?? '—'}
        </span>
        {page.page_type && (
          <span
            className={cn(
              'text-xs rounded px-1.5 py-0.5 font-medium',
              typeColor,
            )}
          >
            {page.page_type}
          </span>
        )}
      </div>
      {Array.isArray(page.data_sources) && page.data_sources.length > 0 && (
        <div>
          <span className="text-xs text-muted-foreground uppercase tracking-wide">
            Data sources
          </span>
          <div className="flex flex-wrap gap-1 mt-1">
            {page.data_sources.map((ds: string, i: number) => (
              <ConstraintBadge key={i} label={ds} />
            ))}
          </div>
        </div>
      )}
      {Array.isArray(page.navigation) && page.navigation.length > 0 && (
        <div>
          <span className="text-xs text-muted-foreground uppercase tracking-wide">
            Navigation
          </span>
          <div className="flex flex-wrap gap-1 mt-1">
            {page.navigation.map((nav: string, i: number) => (
              <ConstraintBadge key={i} label={nav} />
            ))}
          </div>
        </div>
      )}
    </div>
  );
};

const PageCards: React.FC<{ pages: any[] }> = ({ pages }) => (
  <div className="pt-3 space-y-3">
    {pages.map((page, i) => (
      <PageCard key={i} page={page} />
    ))}
  </div>
);

// ── 6. TraceabilityMatrix ─────────────────────────────────────────────────────

const TraceabilityMatrix: React.FC<{ rows: any[] }> = ({ rows }) => (
  <div className="pt-3 overflow-x-auto">
    <table className="w-full border-collapse">
      <thead>
        <tr className="border-b border-border">
          <Th>Use Case</Th>
          <Th>Operations</Th>
          <Th>Pages</Th>
        </tr>
      </thead>
      <tbody>
        {rows.map((row, i) => (
          <tr key={i} className="border-b border-border last:border-0">
            <Td className="font-medium">{row.use_case ?? row.useCase ?? '—'}</Td>
            <Td>
              <div className="flex flex-wrap gap-1">
                {Array.isArray(row.operations)
                  ? row.operations.map((op: string, oi: number) => (
                      <ConstraintBadge key={oi} label={op} />
                    ))
                  : '—'}
              </div>
            </Td>
            <Td>
              <div className="flex flex-wrap gap-1">
                {Array.isArray(row.pages)
                  ? row.pages.map((p: string, pi: number) => (
                      <ConstraintBadge key={pi} label={p} />
                    ))
                  : '—'}
              </div>
            </Td>
          </tr>
        ))}
      </tbody>
    </table>
  </div>
);

// ── BuildSpecStructuredView ───────────────────────────────────────────────────

export const BuildSpecStructuredView: React.FC<BuildSpecStructuredViewProps> = ({
  buildSpec,
}) => {
  if (!buildSpec) {
    return (
      <div className="flex items-center justify-center h-full text-sm text-muted-foreground">
        No Build Spec available.
      </div>
    );
  }

  const project = buildSpec.project;
  const audiences: any[] = buildSpec.auth?.audiences ?? [];
  const entities: any[] = buildSpec.data_model?.entities ?? [];
  const resources: any[] = buildSpec.api?.resources ?? [];
  const pages: any[] = buildSpec.ui?.pages ?? [];
  const traceability: any[] = buildSpec.traceability ?? [];

  return (
    <div className="space-y-3 p-4 overflow-y-auto">
      {/* 1. Project header */}
      {project && (
        <CollapsibleSection title="Project">
          <ProjectHeader project={project} />
        </CollapsibleSection>
      )}

      {/* 2. Auth audiences */}
      {audiences.length > 0 && (
        <CollapsibleSection
          title="Auth"
          badge={`${audiences.length} audience${audiences.length !== 1 ? 's' : ''}`}
        >
          <AuthTable audiences={audiences} />
        </CollapsibleSection>
      )}

      {/* 3. Data model entities */}
      {entities.length > 0 && (
        <CollapsibleSection
          title="Data Model"
          badge={`${entities.length} ${entities.length === 1 ? 'entity' : 'entities'}`}
        >
          <EntityCards entities={entities} />
        </CollapsibleSection>
      )}

      {/* 4. API resources */}
      {resources.length > 0 && (
        <CollapsibleSection
          title="API"
          badge={`${resources.length} resource${resources.length !== 1 ? 's' : ''}`}
        >
          <ApiOperationTree resources={resources} />
        </CollapsibleSection>
      )}

      {/* 5. UI pages */}
      {pages.length > 0 && (
        <CollapsibleSection
          title="UI"
          badge={`${pages.length} page${pages.length !== 1 ? 's' : ''}`}
        >
          <PageCards pages={pages} />
        </CollapsibleSection>
      )}

      {/* 6. Traceability */}
      {traceability.length > 0 && (
        <CollapsibleSection
          title="Traceability"
          badge={`${traceability.length} mapping${traceability.length !== 1 ? 's' : ''}`}
        >
          <TraceabilityMatrix rows={traceability} />
        </CollapsibleSection>
      )}
    </div>
  );
};
