# Session capture NFRs

Inherit existing source path/Unicode/immutable-root boundaries, maxNoteBytes/maxFiles/maxScanBytes/maxResponseBytes, external backup and readonly policies.

Additional acceptance: bounded topic/item/candidate counts; cancellation during scanning; no unbounded model or network work; explicit partial batch results; no loss of unrelated note content; re-capture and restart recovery.

Security baseline disabled; product security constraints remain enforced. Resiliency-01/02/05/09/10/11/13/14/15 apply through bounded local work, receipt visibility, backups and failure tests. Resiliency-03/04 use existing local build/recovery practice; external release is not requested. Resiliency-06/07/08/12 are N/A for distributed health/routing/replication in this local unit.

PBT partial: PBT-02/03/07/08/09 enforced. Pure rendering/ranking properties use existing fast-check with shrinking and seed; other PBT rules are advisory and idempotence is also exercised.

