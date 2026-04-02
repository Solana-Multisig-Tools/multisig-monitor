/**
 * Vercel Edge Function entry point.
 * Re-exports the shared handler for the /api/webhook route.
 */
import handler from "../src/index.js";

export const config = { runtime: "edge" };

export default async function (request: Request): Promise<Response> {
  // Vercel Edge Functions receive the env via process.env
  const env = {
    MULTISIG_ADDRESSES: process.env["MULTISIG_ADDRESSES"],
    HELIUS_AUTH_TOKEN: process.env["HELIUS_AUTH_TOKEN"],
    SLACK_WEBHOOK_URL: process.env["SLACK_WEBHOOK_URL"],
    INSTRUCTION_TYPES: process.env["INSTRUCTION_TYPES"],
    CONFIG_TYPES: process.env["CONFIG_TYPES"],
  };

  // Rewrite path so the shared handler sees /webhook instead of /api/webhook
  const url = new URL(request.url);
  url.pathname = url.pathname.replace(/^\/api/, "");
  const rewritten = new Request(url.toString(), request);

  return handler.fetch(rewritten, env);
}
