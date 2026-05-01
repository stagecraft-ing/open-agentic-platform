# Business Requirements Document

<!--
Synthetic CFS-shaped fabrication fixture for spec 121 SC-001.

This BRD is synthetic — it replicates the STRUCTURAL pattern of the
operator's actual CFS forensic (1GX / Treasury Board / Oracle ERP
fabrication) without copying any private content. The corpus alongside
this file (.artifacts/extracted/business-case.txt) explicitly says
payment processing is OUT OF SCOPE; the validator audit MUST therefore
classify STK-13, INT-003, and SN-022 as Rejected.
-->

## 3. Stakeholders and Scope Constraints

### STK-13 Treasury Board / 1GX Oracle ERP

Treasury Board Integrations operate the 1GX Oracle ERP payment system of record.
All payment transactions route through 1GX for central disbursement.

### INT-003 1GX Integration

The portal integrates with 1GX for payment processing and Oracle ERP reporting.
No direct database access; all calls go through the 1GX API gateway.

### SN-022 1GX Scope Inclusion

1GX integration is in scope for Phase 1 of the funding portal.
Oracle ERP configuration changes are required to support payment flows.
