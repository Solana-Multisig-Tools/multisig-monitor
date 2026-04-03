// ---------------------------------------------------------------------------
// Helius raw webhook payload types
// ---------------------------------------------------------------------------

export interface RawTransaction {
  readonly signature?: string;
  readonly slot?: number;
  readonly blockTime?: number | null;
  readonly indexWithinBlock?: number;
  readonly transaction: TransactionEnvelope;
  readonly meta?: TransactionMeta | null;
}

export interface TransactionEnvelope {
  readonly message: TransactionMessage;
  readonly signatures: readonly string[];
}

export interface TransactionMessage {
  readonly accountKeys: readonly AccountKeyEntry[];
  readonly instructions: readonly CompiledInstruction[];
}

export type AccountKeyEntry =
  | string
  | { readonly pubkey: string; readonly signer?: boolean; readonly writable?: boolean };

export interface CompiledInstruction {
  readonly programIdIndex: number;
  readonly accounts: readonly number[];
  readonly data: string;
}

export interface TransactionMeta {
  readonly err?: unknown;
  readonly innerInstructions?: readonly InnerInstructionSet[];
  readonly logMessages?: readonly string[];
}

export interface InnerInstructionSet {
  readonly index: number;
  readonly instructions: readonly CompiledInstruction[];
}

// ---------------------------------------------------------------------------
// Auth validation
// ---------------------------------------------------------------------------

export function validateAuth(
  authHeader: string | null | undefined,
  expectedToken: string,
): boolean {
  if (!authHeader) return false;
  const encoder = new TextEncoder();
  const a = encoder.encode(authHeader);
  const b = encoder.encode(expectedToken);
  return timingSafeEqual(a, b);
}

function timingSafeEqual(a: Uint8Array, b: Uint8Array): boolean {
  if (a.byteLength !== b.byteLength) return false;

  if (typeof crypto.subtle.timingSafeEqual === "function") {
    return crypto.subtle.timingSafeEqual(a, b);
  }

  let diff = 0;
  for (let i = 0; i < a.byteLength; i++) {
    diff |= a[i]! ^ b[i]!;
  }
  return diff === 0;
}

// ---------------------------------------------------------------------------
// Payload parsing
// ---------------------------------------------------------------------------

export function parsePayload(body: string): RawTransaction[] {
  const parsed: unknown = JSON.parse(body);
  if (!Array.isArray(parsed)) {
    throw new Error("webhook payload must be a JSON array");
  }
  return parsed as RawTransaction[];
}
