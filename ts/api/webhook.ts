/**
 * Vercel Edge Function entry point.
 * Reads config from process.env and delegates to the shared webhook handler.
 */
import { handleWebhook } from "../src/handler.js";

export const config = { runtime: "edge" };

export default async function (request: Request): Promise<Response> {
  return handleWebhook(request, {
    MULTISIG_ADDRESSES: process.env["MULTISIG_ADDRESSES"],
    HELIUS_AUTH_TOKEN: process.env["HELIUS_AUTH_TOKEN"],
    SLACK_WEBHOOK_URL: process.env["SLACK_WEBHOOK_URL"],
    INSTRUCTION_TYPES: process.env["INSTRUCTION_TYPES"],
    CONFIG_TYPES: process.env["CONFIG_TYPES"],
  });
}
