import { describe, expect, it } from "vitest";
import { ApiError } from "./api";
import { checkpointLabel, shortHash } from "./format";
import type { ErrorBody } from "./types";

describe("ApiError — stable code/category mapping", () => {
  const make = (over: Partial<ErrorBody>, status = 400, headerMs?: number) =>
    new ApiError({ code: "X", category: "internal", message: "m", retryable: false, ...over }, status, headerMs);

  it("classifies auth codes", () => {
    expect(make({ code: "SESSION_EXPIRED", category: "auth" }, 401).isAuth).toBe(true);
    expect(make({ code: "UNAUTHENTICATED", category: "auth" }, 401).isAuth).toBe(true);
    expect(make({ code: "VALIDATION_FAILED", category: "validation" }).isAuth).toBe(false);
  });

  it("classifies busy/limit codes", () => {
    expect(make({ code: "PROJECT_BUSY", category: "concurrency" }, 409).isBusy).toBe(true);
    expect(make({ code: "RESOURCE_LIMIT", category: "limit" }, 429).isBusy).toBe(true);
    expect(make({ code: "NOT_FOUND", category: "not_found" }, 404).isBusy).toBe(false);
  });

  it("prefers retry_after_ms from body, falls back to Retry-After header", () => {
    expect(make({ retry_after_ms: 2000 }, 409, 5000).retryAfterMs).toBe(2000);
    expect(make({}, 409, 5000).retryAfterMs).toBe(5000);
  });

  it("branches on code, never message", () => {
    const err = make({ code: "APPROVAL_REQUIRED", category: "approval", message: "anything" }, 422);
    expect(err.code).toBe("APPROVAL_REQUIRED");
    expect(err.category).toBe("approval");
  });
});

describe("format helpers", () => {
  it("shortens hashes and keeps short ones", () => {
    expect(shortHash("abcdefghijklmnopqrstuvwxyz")).toBe("abcdefgh…uvwxyz");
    expect(shortHash("short")).toBe("short");
    expect(shortHash(null)).toBe("—");
  });
  it("maps checkpoints to labels", () => {
    expect(checkpointLabel("ready_to_compile")).toBe("Compile");
    expect(checkpointLabel("verified")).toBe("Verified");
    expect(checkpointLabel("unknown_state")).toBe("unknown_state");
  });
});
