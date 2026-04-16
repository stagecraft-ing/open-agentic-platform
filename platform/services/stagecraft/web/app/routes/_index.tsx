import { Link } from "react-router";

export function meta() {
  return [
    { title: "Open Agentic Platform — Governed OS for AI-Native Software Delivery" },
    {
      name: "description",
      content:
        "OAP turns intent into machine-verifiable software. Spec-driven contracts, governed agents, policy-enforced pipelines — unified in one workspace.",
    },
  ];
}

export default function Landing() {
  return (
    <div className="min-h-full bg-white text-gray-900 dark:bg-gray-950 dark:text-gray-100">
      <SiteHeader />
      <main>
        <Hero />
        <Differentiators />
        <HowItWorks />
        <Architecture />
        <FinalCta />
      </main>
      <SiteFooter />
    </div>
  );
}

function SiteHeader() {
  return (
    <header className="sticky top-0 z-30 border-b border-gray-200/60 bg-white/80 backdrop-blur dark:border-gray-800/60 dark:bg-gray-950/80">
      <div className="container mx-auto flex h-14 items-center justify-between px-4">
        <Link to="/" className="flex items-center gap-2 text-sm font-semibold tracking-tight">
          <Logo className="h-5 w-5 text-indigo-500" />
          <span>Open Agentic Platform</span>
        </Link>
        <div className="flex items-center gap-2">
          <Link
            to="/signin"
            className="hidden sm:inline-flex items-center rounded-md px-3 py-1.5 text-sm text-gray-700 hover:text-gray-900 dark:text-gray-300 dark:hover:text-gray-100"
          >
            Sign in
          </Link>
          <Link
            to="/signup"
            className="inline-flex items-center rounded-md bg-indigo-600 px-3 py-1.5 text-sm font-medium text-white shadow-sm hover:bg-indigo-500 focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:ring-offset-2 dark:focus:ring-offset-gray-950"
          >
            Get started
          </Link>
        </div>
      </div>
    </header>
  );
}

function Hero() {
  return (
    <section className="relative overflow-hidden">
      <div
        aria-hidden
        className="pointer-events-none absolute inset-x-0 -top-24 -z-10 h-[28rem] bg-gradient-to-b from-indigo-500/10 via-transparent to-transparent dark:from-indigo-500/15"
      />
      <div className="container mx-auto px-4 pt-20 pb-24 sm:pt-28 sm:pb-32">
        <div className="max-w-4xl">
          <div className="inline-flex items-center gap-2 rounded-full border border-gray-200 bg-white/60 px-3 py-1 text-xs font-medium text-gray-600 backdrop-blur dark:border-gray-800 dark:bg-gray-900/60 dark:text-gray-300">
            <span className="inline-block h-1.5 w-1.5 rounded-full bg-indigo-500" />
            Open Agentic Platform
          </div>
          <h1 className="mt-6 text-4xl font-bold leading-[1.05] tracking-tight sm:text-6xl">
            The governed operating system for{" "}
            <span className="bg-gradient-to-r from-indigo-500 to-fuchsia-500 bg-clip-text text-transparent">
              AI-native software delivery
            </span>
          </h1>
          <p className="mt-6 max-w-2xl text-lg leading-relaxed text-gray-600 dark:text-gray-400">
            OAP turns intent into machine-verifiable software. Compile specs into
            contracts, run agents under a policy kernel, and promote work through
            governed pipelines — all converging on one workspace.
          </p>
          <div className="mt-8 flex flex-wrap items-center gap-3">
            <Link
              to="/signup"
              className="inline-flex items-center rounded-md bg-indigo-600 px-4 py-2.5 text-sm font-medium text-white shadow-sm hover:bg-indigo-500 focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:ring-offset-2 dark:focus:ring-offset-gray-950"
            >
              Get started
            </Link>
            <Link
              to="/signin"
              className="inline-flex items-center rounded-md border border-gray-300 bg-white px-4 py-2.5 text-sm font-medium text-gray-800 shadow-sm hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:ring-offset-2 dark:border-gray-700 dark:bg-gray-900 dark:text-gray-100 dark:hover:bg-gray-800 dark:focus:ring-offset-gray-950"
            >
              Sign in
            </Link>
          </div>
          <dl className="mt-10 grid max-w-2xl grid-cols-1 gap-x-8 gap-y-3 text-sm sm:grid-cols-3">
            <Metric label="Spec-driven" value="Every feature is a compiled contract" />
            <Metric label="Policy-enforced" value="Gates with proof chains, not vibes" />
            <Metric label="Audit-complete" value="Every agent action, recorded" />
          </dl>
        </div>
      </div>
    </section>
  );
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div className="border-l border-gray-200 pl-4 dark:border-gray-800">
      <dt className="text-xs font-semibold uppercase tracking-wider text-indigo-600 dark:text-indigo-400">
        {label}
      </dt>
      <dd className="mt-1 text-gray-700 dark:text-gray-300">{value}</dd>
    </div>
  );
}

function Differentiators() {
  const items = [
    {
      title: "Spec Spine",
      body: "Human-written specifications compile into a machine-verifiable registry. Every feature is traceable from intent → contract → code. No more drift between design and delivery.",
    },
    {
      title: "Two-plane architecture",
      body: "A web governance plane for approvals, policy, and audit. A desktop execution plane for generation, git, and local tooling. Govern from the cloud, execute where the code lives.",
    },
    {
      title: "Workspace as atom",
      body: "Identity, knowledge, policy, projects, and factories converge on one unit — the workspace. Not a Jira project, a Drive folder, and a pile of runners held together with YAML.",
    },
    {
      title: "Policy kernel, not policy theatre",
      body: "A 5-tier settings merge produces a compiled bundle with proof chains. Every agent call, every gate, every deploy evaluates against the same kernel. Governance is a runtime property.",
    },
    {
      title: "Knowledge as a first-class domain",
      body: "Pluggable connectors (SharePoint, S3, Azure Blob, GCS, upload) normalise documents into a canonical object store. Provenance is preserved; factories consume knowledge, they don't scrape it.",
    },
    {
      title: "Factories with adapters",
      body: "Deterministic multi-stage pipelines for real stacks — Vue/Node, Next/Prisma, Encore/React, Rust/Axum. Consume workspace knowledge, produce build artifacts, ship through governed gates.",
    },
  ];
  return (
    <section className="border-t border-gray-200 dark:border-gray-800">
      <div className="container mx-auto px-4 py-20 sm:py-24">
        <div className="max-w-3xl">
          <p className="text-sm font-semibold uppercase tracking-wider text-indigo-600 dark:text-indigo-400">
            Only one of its class
          </p>
          <h2 className="mt-3 text-3xl font-bold tracking-tight sm:text-4xl">
            Codegen tools ship diffs. OAP ships governed delivery.
          </h2>
          <p className="mt-4 text-gray-600 dark:text-gray-400">
            IDE assistants live inside editors. Agent frameworks live in notebooks. CI
            systems live in YAML. OAP is the first platform that binds them together
            under one spec, one policy kernel, and one audit trail — so AI-written
            software is as governed as anything a human would ship.
          </p>
        </div>
        <div className="mt-12 grid grid-cols-1 gap-px overflow-hidden rounded-xl border border-gray-200 bg-gray-200 sm:grid-cols-2 lg:grid-cols-3 dark:border-gray-800 dark:bg-gray-800">
          {items.map((item) => (
            <div
              key={item.title}
              className="bg-white p-6 dark:bg-gray-950"
            >
              <h3 className="text-base font-semibold text-gray-900 dark:text-gray-100">
                {item.title}
              </h3>
              <p className="mt-2 text-sm leading-relaxed text-gray-600 dark:text-gray-400">
                {item.body}
              </p>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}

function HowItWorks() {
  const steps = [
    {
      n: "01",
      title: "Define",
      body: "Write specs in markdown with machine-readable frontmatter. The spec compiler emits a signed registry that every tool in the system reads from.",
    },
    {
      n: "02",
      title: "Ingest",
      body: "Connect SharePoint, S3, Azure Blob, GCS, or upload directly. Documents are normalised into the workspace's canonical object store with full provenance.",
    },
    {
      n: "03",
      title: "Execute",
      body: "Factories run governed pipelines on the desktop plane. Every stage is typed, every artifact is hashed, every agent call passes through the policy kernel.",
    },
    {
      n: "04",
      title: "Promote",
      body: "Approvals, gates, and deploys flow through the web plane. Reviewers see proof chains, audit sees every action, and production never receives ungoverned bytes.",
    },
  ];
  return (
    <section className="border-t border-gray-200 bg-gray-50 dark:border-gray-800 dark:bg-gray-900/40">
      <div className="container mx-auto px-4 py-20 sm:py-24">
        <div className="max-w-3xl">
          <p className="text-sm font-semibold uppercase tracking-wider text-indigo-600 dark:text-indigo-400">
            How it works
          </p>
          <h2 className="mt-3 text-3xl font-bold tracking-tight sm:text-4xl">
            Intent → contract → artifact → deployment.
          </h2>
        </div>
        <ol className="mt-12 grid grid-cols-1 gap-6 sm:grid-cols-2 lg:grid-cols-4">
          {steps.map((s) => (
            <li
              key={s.n}
              className="relative rounded-xl border border-gray-200 bg-white p-6 dark:border-gray-800 dark:bg-gray-950"
            >
              <div className="font-mono text-xs text-indigo-600 dark:text-indigo-400">
                {s.n}
              </div>
              <h3 className="mt-2 text-base font-semibold">{s.title}</h3>
              <p className="mt-2 text-sm leading-relaxed text-gray-600 dark:text-gray-400">
                {s.body}
              </p>
            </li>
          ))}
        </ol>
      </div>
    </section>
  );
}

function Architecture() {
  return (
    <section className="border-t border-gray-200 dark:border-gray-800">
      <div className="container mx-auto px-4 py-20 sm:py-24">
        <div className="max-w-3xl">
          <p className="text-sm font-semibold uppercase tracking-wider text-indigo-600 dark:text-indigo-400">
            Architecture
          </p>
          <h2 className="mt-3 text-3xl font-bold tracking-tight sm:text-4xl">
            Two planes. One system. Zero lock-in.
          </h2>
          <p className="mt-4 text-gray-600 dark:text-gray-400">
            The web plane governs. The desktop plane executes. They share one
            workspace model, one policy kernel, and one audit log — so work moves
            between them without translation.
          </p>
        </div>
        <div className="mt-12 grid grid-cols-1 gap-6 lg:grid-cols-2">
          <PlaneCard
            tag="Governance plane"
            title="Stagecraft"
            description="Runs in the cloud. Identity, workspace administration, knowledge intake, approvals, deploy promotion, and audit. Read-heavy surface for reviewers, operators, and compliance."
            bullets={[
              "GitHub OAuth + OIDC identity",
              "Workspace-scoped knowledge object store",
              "Policy bundle serving & grant management",
              "Deploy orchestration to Azure Kubernetes",
            ]}
          />
          <PlaneCard
            tag="Execution plane"
            title="OPC desktop"
            description="Runs locally. Factories, code generation, git operations, checkpoints, and inspection — enforcing the same policy kernel Stagecraft serves. Works offline; reconciles when online."
            bullets={[
              "Multi-agent orchestrator with DAG validation",
              "Unified MCP agent (axiomregent)",
              "Checkpointed, reversible execution",
              "Streamed audit back to Stagecraft",
            ]}
          />
        </div>
      </div>
    </section>
  );
}

function PlaneCard({
  tag,
  title,
  description,
  bullets,
}: {
  tag: string;
  title: string;
  description: string;
  bullets: string[];
}) {
  return (
    <div className="rounded-xl border border-gray-200 bg-white p-6 dark:border-gray-800 dark:bg-gray-950">
      <div className="flex items-center gap-2 text-xs font-semibold uppercase tracking-wider text-indigo-600 dark:text-indigo-400">
        <span className="inline-block h-1.5 w-1.5 rounded-full bg-indigo-500" />
        {tag}
      </div>
      <h3 className="mt-2 text-xl font-semibold">{title}</h3>
      <p className="mt-2 text-sm leading-relaxed text-gray-600 dark:text-gray-400">
        {description}
      </p>
      <ul className="mt-4 space-y-2 text-sm">
        {bullets.map((b) => (
          <li key={b} className="flex items-start gap-2 text-gray-700 dark:text-gray-300">
            <Check className="mt-0.5 h-4 w-4 flex-none text-indigo-500" />
            <span>{b}</span>
          </li>
        ))}
      </ul>
    </div>
  );
}

function FinalCta() {
  return (
    <section className="border-t border-gray-200 dark:border-gray-800">
      <div className="container mx-auto px-4 py-20 sm:py-24">
        <div className="rounded-2xl border border-gray-200 bg-gradient-to-br from-indigo-50 via-white to-white p-10 dark:border-gray-800 dark:from-indigo-500/10 dark:via-gray-950 dark:to-gray-950">
          <div className="max-w-2xl">
            <h2 className="text-3xl font-bold tracking-tight sm:text-4xl">
              Govern your agents. Ship verified software.
            </h2>
            <p className="mt-4 text-gray-600 dark:text-gray-400">
              Create a workspace, connect your knowledge sources, and run your first
              governed factory pipeline today.
            </p>
            <div className="mt-6 flex flex-wrap items-center gap-3">
              <Link
                to="/signup"
                className="inline-flex items-center rounded-md bg-indigo-600 px-4 py-2.5 text-sm font-medium text-white shadow-sm hover:bg-indigo-500 focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:ring-offset-2 dark:focus:ring-offset-gray-950"
              >
                Get started
              </Link>
              <Link
                to="/signin"
                className="inline-flex items-center rounded-md border border-gray-300 bg-white px-4 py-2.5 text-sm font-medium text-gray-800 shadow-sm hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:ring-offset-2 dark:border-gray-700 dark:bg-gray-900 dark:text-gray-100 dark:hover:bg-gray-800 dark:focus:ring-offset-gray-950"
              >
                Sign in
              </Link>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}

function SiteFooter() {
  return (
    <footer className="border-t border-gray-200 dark:border-gray-800">
      <div className="container mx-auto flex flex-col gap-3 px-4 py-8 text-xs text-gray-500 sm:flex-row sm:items-center sm:justify-between dark:text-gray-500">
        <div className="flex items-center gap-2">
          <Logo className="h-4 w-4 text-indigo-500" />
          <span>Open Agentic Platform</span>
        </div>
        <div>© {new Date().getFullYear()} OAP. Governed by design.</div>
      </div>
    </footer>
  );
}

function Logo({ className }: { className?: string }) {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.75"
      strokeLinecap="round"
      strokeLinejoin="round"
      className={className}
      aria-hidden
    >
      <path d="M12 3 3 7.5v9L12 21l9-4.5v-9z" />
      <path d="M3 7.5 12 12l9-4.5" />
      <path d="M12 12v9" />
    </svg>
  );
}

function Check({ className }: { className?: string }) {
  return (
    <svg
      viewBox="0 0 20 20"
      fill="currentColor"
      className={className}
      aria-hidden
    >
      <path
        fillRule="evenodd"
        d="M16.704 5.29a1 1 0 0 1 .006 1.414l-7.5 7.6a1 1 0 0 1-1.42.006L3.29 9.82a1 1 0 1 1 1.42-1.408l3.787 3.82 6.793-6.885a1 1 0 0 1 1.414-.057z"
        clipRule="evenodd"
      />
    </svg>
  );
}
