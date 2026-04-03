import { describe, expect, it } from "vitest";
import { parsePayload, validateAuth } from "./webhook.js";

describe("webhook", () => {
  it("accepts an exact auth token match", () => {
    expect(validateAuth("secret-token", "secret-token")).toBe(true);
  });

  it("rejects a missing auth token", () => {
    expect(validateAuth(undefined, "secret-token")).toBe(false);
    expect(validateAuth(null, "secret-token")).toBe(false);
  });

  it("rejects an auth token with a different length", () => {
    expect(validateAuth("secret-token-extra", "secret-token")).toBe(false);
  });

  it("rejects an auth token with the same length but different content", () => {
    expect(validateAuth("secret-tokfn", "secret-token")).toBe(false);
  });

  it("parses a raw webhook transaction array", () => {
    const parsed = parsePayload(
      JSON.stringify([
        {
          signature: "sig-1",
          transaction: {
            message: {
              accountKeys: ["acct-1"],
              instructions: [],
            },
            signatures: ["sig-1"],
          },
          meta: null,
        },
      ]),
    );

    expect(parsed).toHaveLength(1);
    expect(parsed[0]?.signature).toBe("sig-1");
  });

  it("rejects a non-array webhook payload", () => {
    expect(() => parsePayload(JSON.stringify({ signature: "sig-1" }))).toThrow(
      "webhook payload must be a JSON array",
    );
  });
});
