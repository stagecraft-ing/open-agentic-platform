# Security Policy

## Supported Versions

Open Agentic Platform is pre-1.0. Only the latest commit on the `main` branch receives security fixes. There are no backported patches to older releases or tags.

| Version | Supported |
|---------|-----------|
| main (latest) | Yes |
| any prior tag | No |

---

## Reporting a Vulnerability

Use GitHub's private security advisory feature for this repository:

**GitHub Security Advisories:** https://github.com/[owner]/open-agentic-platform/security/advisories/new

Do NOT open a public issue. Public disclosure of an unpatched vulnerability puts all users at risk.

Include in your report:
- Description of the vulnerability and its potential impact
- Steps to reproduce or a proof-of-concept
- Affected component (crate, service, or subsystem)
- Any suggested mitigation, if you have one

---

## Response Timeline

| Stage | Target |
|-------|--------|
| Acknowledgment | Within 72 hours of report |
| Triage and severity assessment | Within 1 week |
| Fix for critical issues | Within 30 days |
| Fix for high/medium issues | Best effort, communicated to reporter |

If a reported issue requires more time than the target, you will be notified with a revised estimate.

---

## Disclosure Policy

This project follows coordinated disclosure. We will:

1. Confirm receipt and assess severity.
2. Work on a fix and agree on a disclosure timeline with the reporter.
3. Release the fix before or simultaneously with any public disclosure.
4. Credit the reporter in the release notes unless they request anonymity.

We will not take legal action against researchers who report vulnerabilities in good faith and follow this policy.

---

## Agent Security Model

OAP includes a policy kernel with permission tiers, tool allowlists, and governed execution designed to sandbox AI agent behavior. The governance layer — the policy kernel, gate evaluator, and audit trail — is a core security boundary of the system.

Security issues in the governance layer are treated as critical regardless of immediate exploitability. This includes bypasses of permission gates, policy evaluation errors, and audit trail integrity failures.
