/**
 * Optional search-text folding (REQ-015 / BR-FOLD-1). Pure and deterministic:
 * NFC canonical normalization + locale-independent case folding. It is NOT NFKC
 * and never uses `toLocaleLowerCase` (which is locale-dependent, e.g. Turkish İ).
 * Hangul is caseless, so `toLowerCase` is a no-op for it, and NFC is a lossless
 * canonical unification of composed/decomposed forms. Kept in its own module so
 * it can be property-tested and so later lexical search helpers can live beside it.
 */
export function foldText(value: string): string {
  return value.normalize('NFC').toLowerCase();
}
