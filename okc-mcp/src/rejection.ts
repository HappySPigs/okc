/**
 * Canonical rejection taxonomy (functional-design business-rules.md BR-REJECT-1/2,
 * domain-entities.md · Rejection). The Vault/notes layers raise richer,
 * implementation-specific error codes; the MCP surface maps them to this closed
 * `kind` set so tool errors are predictable and testable, while the original
 * `code` is preserved in `detail` for diagnosis.
 */

export type RejectionKind =
  | 'path-denied'
  | 'hash-mismatch'
  | 'malformed-yaml'
  | 'overwrite-refused'
  | 'bounds-exceeded'
  | 'not-found';

export interface Rejection {
  readonly kind: RejectionKind;
  readonly message: string;
  readonly detail?: Record<string, unknown>;
}

/**
 * Map an implementation error code to the canonical design `kind`.
 * Returns `undefined` for operational safety refusals that are intentionally
 * outside the design's enumerated set (e.g. VAULT_BUSY, cleanup failures,
 * config errors) — the surface still reports those, with their `code`, but
 * does not force them into a misleading `kind`.
 */
export function toKind(code: string | undefined): RejectionKind | undefined {
  switch (code) {
    // Path safety / bounded authority (REQ-008, BR-PATH-*).
    case 'INVALID_PATH':
    case 'PATH_ESCAPE':
    case 'PATH_DEPTH':
    case 'PATH_COLLISION':
    case 'PATH_CHANGED':
    case 'SYMLINK':
    case 'HARDLINK':
    case 'NOT_REGULAR_FILE':
    case 'IMMUTABLE_TARGET':
    case 'INVALID_VAULT':
    case 'INVALID_STATE':
    case 'STATE_OVERLAP':
    case 'ROOT_CHANGED':
      return 'path-denied';
    // Conflict-aware writes (REQ-004, BR-HASH-3/4). A concurrent change
    // detected mid-operation is, from the caller's view, a stale-baseline conflict.
    case 'CONFLICT':
    case 'INVALID_HASH':
    case 'FILE_CHANGED':
      return 'hash-mismatch';
    // Structure preservation / malformed YAML (REQ-005, BR-STRUCT-2).
    case 'NOTE_INVALID':
    case 'INVALID_CONTENT':
    case 'INVALID_UTF8':
      return 'malformed-yaml';
    // Create refuses to overwrite (REQ-003, BR-CREATE-1).
    case 'NOTE_EXISTS':
      return 'overwrite-refused';
    // Resource bounds (REQ-008, BR-BOUND-*).
    case 'NOTE_TOO_LARGE':
    case 'NOTE_LIMIT':
    case 'SCAN_LIMIT':
    case 'RESPONSE_LIMIT':
      return 'bounds-exceeded';
    // Absent note.
    case 'NOTE_NOT_FOUND':
      return 'not-found';
    default:
      return undefined;
  }
}
