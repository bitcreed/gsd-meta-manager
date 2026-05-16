---
quick_id: 260515-w3a
status: complete
date: 2026-05-15
---

# Detect SPEC.md and EVAL-REVIEW.md sub-phase artifacts

Mirrors UAT/SECURITY pattern.

- `DiskInference.has_spec`, `DiskInference.has_eval_review`
- Detection branches placed before generic matches and before REVIEW/SPEC peers
  (UI-SPEC, AI-SPEC, UI-REVIEW already filter first; EVAL-REVIEW filters before
  REVIEW; SPEC filters after AI-SPEC and UI-SPEC).
- "Spec" row added at top of Plan sub-stages; "Eval Review" added in Execute
  sub-stages between UI Review and UAT.
- 4 new tests including misclassification negatives (AI-SPEC ≠ SPEC,
  EVAL-REVIEW ≠ REVIEW).

Verification: cargo test — 125 passed.
