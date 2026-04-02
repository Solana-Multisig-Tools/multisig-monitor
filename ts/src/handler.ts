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
    [...appConfig.multisigAddresses],
    appConfig.filter,
  );

  for (const op of operations) {
    try {
      await sendNotification(op, appConfig.slackWebhookUrl);
    } catch (e) {
      console.error("slack notification failed:", e);
    }
  }

  return new Response("ok");
}
