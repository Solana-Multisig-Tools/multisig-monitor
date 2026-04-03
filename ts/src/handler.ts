import { loadConfig, type Env } from "./config.js";
import { sendNotification } from "./slack.js";
import { extractOperations } from "./squads/index.js";
import { parsePayload, validateAuth } from "./webhook.js";

export async function handleWebhook(
  request: Request,
  env: Env,
): Promise<Response> {
  let appConfig;
  try {
    appConfig = loadConfig(env);
  } catch (e) {
    return new Response(`config error: ${e}`, { status: 500 });
  }

  const authHeader = request.headers.get("Authorization");
  if (!validateAuth(authHeader, appConfig.heliusAuthToken)) {
    return new Response("unauthorized", { status: 401 });
  }

  const MAX_BODY_SIZE = 5 * 1024 * 1024; // 5 MB
  const contentLength = request.headers.get("content-length");
  if (contentLength && parseInt(contentLength, 10) > MAX_BODY_SIZE) {
    return new Response("payload too large", { status: 413 });
  }

  let body: string;
  try {
    body = await request.text();
  } catch {
    return new Response("bad request", { status: 400 });
  }

  if (body.length > MAX_BODY_SIZE) {
    return new Response("payload too large", { status: 413 });
  }

  let transactions;
  try {
    transactions = parsePayload(body);
  } catch (e) {
    return new Response(`bad request: ${e}`, { status: 400 });
  }

  const operations = await extractOperations(
    transactions,
    [...appConfig.multisigAddresses],
    appConfig.filter,
  );

  const results = await Promise.allSettled(
    operations.map((op) => sendNotification(op, appConfig.slackWebhookUrl, appConfig.display)),
  );
  for (const result of results) {
    if (result.status === "rejected") {
      console.error("slack notification failed:", result.reason);
    }
  }

  return new Response("ok");
}
