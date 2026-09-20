/**
 * Shared frontend error humanization.
 *
 * Tauri command errors can reach the UI in two shapes:
 *
 * 1. A thrown `Error` whose `.message` is the JSON stringification of the
 *    structured Rust `CommandError`, e.g.
 *    `Error: {"kind":"validation","detail":{"message":"…"}}`.
 * 2. A direct object already shaped like
 *    `{ kind: "validation", detail: { message: "…" } }` (or
 *    `{ message: "…" }`).
 *
 * Plain JS `String(e)` falls back to `"[object Object]"` for the second shape
 * and emits the noisy `"Error: …"` prefix for the first, both of which are
 * unfit for user-facing toasts and inline error labels.
 *
 * `humanizeError` accepts `unknown` (any thrown value) and returns a clean,
 * user-displayable string. It collapses both error shapes to the inner
 * `detail.message` (or top-level `message`) when present, and falls back to
 * the raw string for plain `Error.message` text that is not JSON. Unknown
 * shapes (objects with no recognizable message field, `null`, `undefined`)
 * return an empty string instead of `"[object Object]"` so the caller can
 * compose it without a defensive guard.
 */

/**
 * Extract a human message from a string that may or may not be the JSON
 * serialization of a structured Tauri `CommandError`.
 *
 * Strings that do not start with `{` or `[` are returned verbatim (the
 * fast-path keeps normal `Error.message` text intact). Otherwise the string
 * is parsed and, if it has a `detail.message` or top-level `message`, that
 * inner value is returned. JSON-shaped strings that fail to parse, or parse
 * to a value without a recognizable message, fall back to the raw string.
 */
function extractFromString(raw: string): string {
  if (!raw) return "";
  const trimmed = raw.trim();
  // Fast path: only attempt JSON.parse on values that look like a serialized
  // object/array. Normal `Error.message` text (e.g. "uniqueness violation: …")
  // skips the parse and is returned as-is.
  if (!trimmed.startsWith("{") && !trimmed.startsWith("[")) {
    return raw;
  }
  try {
    const parsed: unknown = JSON.parse(trimmed);
    const inner = messageFromObject(parsed);
    if (inner) return inner;
  } catch {
    // Not JSON, or malformed JSON — fall through to the raw string.
  }
  return raw;
}

/**
 * Walk a parsed JSON value (or a direct object thrown by Tauri) looking for a
 * `detail.message` or top-level `message` string. Returns the empty string
 * when no usable message is found so the caller can compose the result
 * without a defensive check.
 */
function messageFromObject(value: unknown): string {
  if (!value || typeof value !== "object") return "";
  const obj = value as Record<string, unknown>;
  const detail = obj.detail;
  if (detail && typeof detail === "object") {
    const detailMsg = (detail as Record<string, unknown>).message;
    if (typeof detailMsg === "string" && detailMsg.length > 0) return detailMsg;
  }
  const top = obj.message;
  if (typeof top === "string" && top.length > 0) return top;
  return "";
}

/**
 * Convert any thrown value (or any other unknown) into a user-displayable
 * string. See the file header for the full rationale.
 *
 * - `null` / `undefined` → `""`
 * - `Error` instance → `e.message` run through the JSON unwrap heuristic
 * - string → unwrapped (JSON `detail.message`/`message` if present, otherwise raw)
 * - plain object → `detail.message` or `message` if present, else `""`
 * - everything else → `""`
 */
export function humanizeError(e: unknown): string {
  if (e === null || e === undefined) return "";

  if (e instanceof Error) {
    // Prefer the structured walk for an Error whose `message` happens to be
    // a JSON string (the Tauri CommandError case). Fall back to the raw
    // `e.message` so plain Error messages like "Network unreachable" survive
    // even when the heuristic returns empty.
    const unwrapped = extractFromString(e.message);
    if (unwrapped) return unwrapped;
    return e.message ?? "";
  }

  if (typeof e === "string") {
    return extractFromString(e);
  }

  if (typeof e === "object") {
    return messageFromObject(e);
  }

  return "";
}
