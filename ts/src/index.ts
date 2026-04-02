import { loadConfig, type Env } from "./config.js";
import { sendNotification } from "./slack.js";
import { extractOperations } from "./squads/index.js";
import { parsePayload, validateAuth } from "./webhook.js";

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);

    if (request.method === "GET" && url.pathname === "/health") {
      return new Response("ok");
    }

    if (request.method === "POST" && url.pathname === "/webhook") {
      return handleWebhook(request, env);
    }

    return new Response("not found", { status: 404 });
  },
};

async function handleWebhook(request: Request, env: Env): Promise<Response> {
  let config;
  try {
    config = loadConfig(env);
  } catch (e) {
    return new Response(`config error: ${e}`, { status: 500 });
  }

  const authHeader = request.headers.get("Authorization");
  if (!validateAuth(authHeader, config.heliusAuthToken)) {
    return new Response("unauthorized", { status: 401 });
  }

  let body: string;
  try {
    body = await request.text();
  } catch {
    return new Response("bad request", { status: 400 });
  }

  let transactions;
  try {
    transactions = parsePayload(body);
  } catch (e) {
    return new Response(`bad request: ${e}`, { status: 400 });
  }

  const operations = await extractOperations(
    transactions,
    [...config.multisigAddresses],
    config.filter,
  );

  for (const op of operations) {
    try {
      await sendNotification(op, config.slackWebhookUrl);
    } catch (e) {
      console.error("slack notification failed:", e);
    }
  }

  return new Response("ok");
}

// Vercel Edge Functions export
export const config = {
  runtime: "edge",
};
