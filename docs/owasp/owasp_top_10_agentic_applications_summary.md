# OWASP Top 10 for Agentic Applications (ASI 2026) — Executive Summary

The **OWASP Top 10 for Agentic Applications (ASI)** is a specialized security framework designed to address the unique vulnerabilities of autonomous and semi-autonomous AI systems. 

Unlike traditional LLM applications that focus primarily on text generation, agentic AI systems reason, rely on persistent memory, execute multi-step plans, and invoke external tools autonomously—introducing an entirely new and complex attack surface.

---

### The Top 10 Agentic AI Risks (ASI 2026)

* **ASI01: Agent Goal Hijack** — Attackers manipulate agent planning loops or core objectives using direct or indirect prompt instruction injections, forcing the agent to pursue malicious targets.
* **ASI02: Tool Misuse & Exploitation** — Agents with overly broad permissions are tricked or hallucinate into invoking valid system tools in unsafe, unintended, or highly destructive ways.
* **ASI03: Identity & Privilege Abuse** — Attackers exploit cached credentials, shared long-lived tokens, or improperly scoped service accounts to escalate privileges across cloud and platform boundaries.
* **ASI04: Agentic Supply Chain** — Vulnerabilities introduced by dynamically resolving or loading untrusted third-party agent components, custom personas, or unverified external tool plugins.
* **ASI05: Unexpected Code Execution / RCE** — Attackers trick automated development or "vibecoding" agents into generating, downloading, or executing malicious scripts inside live runtime environments.
* **ASI06: Memory & Context Poisoning** — Direct manipulation of RAG vector knowledge bases or an agent's long-term episodic memory to permanently warp its reasoning and decision logic.
* **ASI07: Insecure Inter-Agent Communication** — Spoofing, intercepting, or modifying messages passed between cooperating sub-agents over unencrypted, unauthenticated message buses or queues.
* **ASI08: Cascading Failures** — A single unhandled hallucination, validation failure, or compromised task ripples exponentially through a chain of autonomous agents, leading to rapid system-wide failure.
* **ASI09: Human-Agent Trust Exploitation** — Cybercriminals leverage human anthropomorphism, manipulating users or operators into blindly approving malicious actions or bypassing structural security controls.
* **ASI10: Rogue Agents** — Compromised, unconstrained, or hallucinating agents that veer entirely off-assignment, self-replicate, or spawn hidden background processes to pursue rogue agendas.

---

### Actionable Mitigations

To safely build and deploy agentic AI platforms, teams should standardize on the following core controls:

1.  **Least Agency & Privilege:** Strictly limit the operational degrees of freedom granted to an agent. Restrict tool access to minimal, well-typed parameters, and default to read-only permissions wherever possible.
2.  **Human-in-the-Loop (HITL):** Mandate explicit, out-of-band human verification and cryptographic approval for any high-risk, non-deterministic, or irreversible state changes (e.g., mass data mutations, deployment modifications, or financial actions).
3.  **Execution Isolation:** Decouple and execute all dynamic code blocks, tool invocations, and generated scripts inside strictly isolated, ephemeral micro-sandbox environments (e.g., hardened containers with rigid network policies and low time-to-live thresholds).
