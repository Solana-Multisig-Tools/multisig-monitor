import type { DetectedOperation } from "./squads/index.js";
import type { SquadsOperation, ConfigChange } from "./squads/types.js";
import {
  displayName,
  emoji,
  memberRole,
  configChangeDescription,
  permissionLabels,
} from "./squads/types.js";

// ---------------------------------------------------------------------------
// Slack Block Kit types
// ---------------------------------------------------------------------------

interface SlackPayload {
  readonly blocks: readonly SlackBlock[];
}

type SlackBlock =
  | { readonly type: "header"; readonly text: { readonly type: "plain_text"; readonly text: string } }
  | { readonly type: "section"; readonly text: { readonly type: "mrkdwn"; readonly text: string } }
  | { readonly type: "divider" };

function header(text: string): SlackBlock {
  return { type: "header", text: { type: "plain_text", text } };
}

function section(text: string): SlackBlock {
  return { type: "section", text: { type: "mrkdwn", text } };
}

function divider(): SlackBlock {
  return { type: "divider" };
}

// ---------------------------------------------------------------------------
// Payload building
// ---------------------------------------------------------------------------

export function buildPayload(op: DetectedOperation): SlackPayload {
  const blocks: SlackBlock[] = [
    header(`${emoji(op.operation)} ${displayName(op.operation)}`),
    section(
      `*Multisig:* \`${op.multisig}\`\n*${memberRole(op.operation)}:* \`${op.member}\``,
    ),
  ];

  // Transaction and proposal addresses for correlation
  const refLines: string[] = [];
  if (op.transactionAddress) {
    refLines.push(`*Transaction:* \`${op.transactionAddress}\``);
  }
  if (op.proposalAddress) {
    refLines.push(`*Proposal:* \`${op.proposalAddress}\``);
  }
  if (refLines.length > 0) {
    blocks.push(section(refLines.join("\n")));
  }

  // Operation-specific details
  appendOperationDetails(blocks, op.operation);

  // Transaction signature with Solscan link
  blocks.push(
    section(`*Tx:* <https://solscan.io/tx/${op.signature}|${op.signature}>`),
  );

  blocks.push(divider());

  return { blocks };
}

function appendOperationDetails(
  blocks: SlackBlock[],
  operation: SquadsOperation,
): void {
  switch (operation.kind) {
    case "config_transaction_create": {
      if (operation.actions.length > 0) {
        const lines = operation.actions.map(
          (a) => `\u2022 ${configChangeDescription(a)}`,
        );
        blocks.push(section(`*Config Actions:*\n${lines.join("\n")}`));
      }
      break;
    }

    case "multisig_create": {
      const memberLines = operation.members.map(
        (m) =>
          `\u2022 \`${m.key}\` (${permissionLabels(m.permissions).join(", ")})`,
      );
      const authority = operation.configAuthority ?? "None (autonomous)";
      blocks.push(
        section(
          `*Threshold:* ${operation.threshold}\n*Config Authority:* ${authority}\n*Members:*\n${memberLines.join("\n")}`,
        ),
      );
      break;
    }

    case "multisig_add_member": {
      const perms = permissionLabels(operation.newMember.permissions).join(
        ", ",
      );
      blocks.push(
        section(
          `*New Member:* \`${operation.newMember.key}\`\n*Permissions:* ${perms}`,
        ),
      );
      break;
    }

    case "multisig_remove_member":
      blocks.push(section(`*Removed Member:* \`${operation.oldMember}\``));
      break;

    case "multisig_change_threshold":
      blocks.push(section(`*New Threshold:* ${operation.newThreshold}`));
      break;

    case "multisig_set_time_lock":
      blocks.push(section(`*Time Lock:* ${operation.timeLock}s`));
      break;

    case "multisig_set_config_authority":
      blocks.push(
        section(
          `*New Config Authority:* \`${operation.configAuthority}\``,
        ),
      );
      break;

    case "multisig_set_rent_collector": {
      const value = operation.rentCollector
        ? `\`${operation.rentCollector}\``
        : "None (removed)";
      blocks.push(section(`*Rent Collector:* ${value}`));
      break;
    }

    case "multisig_add_spending_limit":
      blocks.push(
        section(
          `*Amount:* ${operation.amount}\n*Mint:* \`${operation.mint}\`\n*Vault:* ${operation.vaultIndex}`,
        ),
      );
      break;

    case "spending_limit_use":
      blocks.push(
        section(
          `*Amount:* ${operation.amount} (decimals: ${operation.decimals})`,
        ),
      );
      break;

    case "proposal_create":
      if (operation.draft) {
        blocks.push(section("*Status:* Draft"));
      }
      break;

    default:
      break;
  }
}

// ---------------------------------------------------------------------------
// Send notification
// ---------------------------------------------------------------------------

export async function sendNotification(
  op: DetectedOperation,
  webhookUrl: string,
): Promise<void> {
  const payload = buildPayload(op);

  const resp = await fetch(webhookUrl, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(payload),
  });

  if (!resp.ok) {
    throw new Error(`slack returned status ${resp.status}`);
  }
}
