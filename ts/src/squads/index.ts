import bs58 from "bs58";
import type { NotificationFilter } from "../filter.js";
import { allowsInstruction, applyFilter } from "../filter.js";
import type { RawTransaction } from "../webhook.js";
import { layoutFor, resolvePubkey } from "./accounts.js";
import { buildOperation } from "./decoder.js";
import type { InstructionKind } from "./discriminator.js";
import { getDiscriminatorMap, matchDiscriminator } from "./discriminator.js";
import type { DetectedOperation } from "./types.js";
import { SQUADS_PROGRAM_ID } from "./types.js";

export type { DetectedOperation } from "./types.js";
export type { InstructionKind } from "./discriminator.js";

/**
 * Extracts all Squads v4 operations from a batch of raw transactions,
 * filtered to only the configured multisig addresses and matching the notification filter.
 */
export async function extractOperations(
  transactions: readonly RawTransaction[],
  monitoredMultisigs: readonly string[],
  filter: NotificationFilter,
): Promise<DetectedOperation[]> {
  const discMap = await getDiscriminatorMap();
  const results: DetectedOperation[] = [];

  for (const tx of transactions) {
    if (tx.meta?.err) continue;

    const blockTime = tx.blockTime ?? null;

    const signature =
      tx.signature ?? tx.transaction.signatures[0] ?? undefined;
    if (!signature) continue;

    const accountKeys = tx.transaction.message.accountKeys.map((k) =>
      typeof k === "string" ? k : k.pubkey,
    );

    for (const instr of tx.transaction.message.instructions) {
      const op = processInstruction(
        instr,
        accountKeys,
        signature,
        monitoredMultisigs,
        discMap,
        filter,
        blockTime,
      );
      if (op) results.push(op);
    }

    if (tx.meta?.innerInstructions) {
      for (const set of tx.meta.innerInstructions) {
        for (const instr of set.instructions) {
          const op = processInstruction(
            instr,
            accountKeys,
            signature,
            monitoredMultisigs,
            discMap,
            filter,
            blockTime,
          );
          if (op) results.push(op);
        }
      }
    }
  }

  return results;
}

interface CompiledInstruction {
  readonly programIdIndex: number;
  readonly accounts: readonly number[];
  readonly data: string;
}

function processInstruction(
  instr: CompiledInstruction,
  accountKeys: readonly string[],
  signature: string,
  monitoredMultisigs: readonly string[],
  discMap: Map<string, InstructionKind>,
  filter: NotificationFilter,
  blockTime: number | null,
): DetectedOperation | null {
  const programId = accountKeys[instr.programIdIndex];
  if (programId !== SQUADS_PROGRAM_ID) return null;

  let data: Uint8Array;
  try {
    data = bs58.decode(instr.data);
  } catch {
    return null;
  }

  const kind = matchDiscriminator(data, discMap);
  if (!kind) return null;

  if (!allowsInstruction(filter, kind)) return null;

  const layout = layoutFor(kind);
  const multisig = resolvePubkey(
    instr.accounts,
    accountKeys,
    layout.multisigIndex,
  );
  if (!multisig || !monitoredMultisigs.includes(multisig)) return null;

  const member = resolvePubkey(
    instr.accounts,
    accountKeys,
    layout.memberIndex,
  );
  if (!member) return null;

  const transactionAddress =
    layout.transactionIndex !== null
      ? resolvePubkey(instr.accounts, accountKeys, layout.transactionIndex)
      : null;

  const proposalAddress =
    layout.proposalIndex !== null
      ? resolvePubkey(instr.accounts, accountKeys, layout.proposalIndex)
      : null;

  const operation = buildOperation(kind, data);
  if (!operation) return null;

  const filtered = applyFilter(filter, operation);
  if (!filtered) return null;

  return {
    signature,
    multisig,
    member,
    transactionAddress,
    proposalAddress,
    operation: filtered,
    blockTime,
  };
}
