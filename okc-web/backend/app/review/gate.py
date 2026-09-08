"""``review.gate`` — the DecisionGate (gate-before-core, C-1 analogue / C-2).

Runs entirely in okc-web BEFORE any engine op, so a rejected cluster approval reaches
zero reserving/mutating engine calls and writes zero audit rows. Necessity is
code-verified: okc-core's blocking-approval rejection surfaces as the generic
``PROJECT_INVALID`` (not ``APPROVAL_REQUIRED``), so the deterministic 422 must originate
here. Pure functions over already-parsed ``CriticFindingView`` — trivially unit-testable
offline (no engine, no LLM).
"""

from __future__ import annotations

from app.review.models import ApproveClusterRequest, CriticFindingView
from app.shared.error import EngineError, EngineErrorCategory, EngineErrorCode


def _approval_required(message: str) -> EngineError:
    return EngineError(EngineErrorCode.APPROVAL_REQUIRED, EngineErrorCategory.APPROVAL, message)


class DecisionGate:
    """The okc-web review-domain gate. All methods raise ``EngineError`` on rejection."""

    @staticmethod
    def assert_rationales(cmd: ApproveClusterRequest) -> None:
        """Body-only (no findings needed): every Minor waiver and omission proposal MUST
        carry a non-blank rationale (E4-S3, "사유 필수") → ``VALIDATION_FAILED`` (400)."""
        for finding_id, rationale in cmd.minor_waivers.items():
            if not rationale.strip():
                raise EngineError.validation(
                    f"minor waiver for finding '{finding_id}' requires a rationale"
                )
        for key, rationale in cmd.omission_rationales.items():
            if not rationale.strip():
                raise EngineError.validation(
                    f"omission proposal for '{key}' requires a rationale"
                )

    @staticmethod
    def assert_approvable(findings: list[CriticFindingView], cmd: ApproveClusterRequest) -> None:
        """Blocking-finding rules (C-2) → ``APPROVAL_REQUIRED`` (422):
        1. an explicit attempt to waive a Major/Critical finding, and
        2. any residual blocking finding on the cluster (not approvable at all).
        Blocking findings are resolved only by regenerate."""
        by_id = {f.finding_id: f for f in findings}
        for finding_id in cmd.minor_waivers:
            f = by_id.get(finding_id)
            if f is not None and f.blocking:
                raise _approval_required(
                    f"finding '{finding_id}' is {f.severity} and cannot be waived; "
                    "regenerate the cluster to resolve blocking findings (C-2)"
                )
        blocking = [f.finding_id for f in findings if f.blocking]
        if blocking:
            raise _approval_required(
                f"cluster has blocking (Major/Critical) findings {blocking}; "
                "resolve via regenerate before approval (C-2)"
            )
