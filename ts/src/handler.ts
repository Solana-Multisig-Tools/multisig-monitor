import { loadConfig, resolveWebhookUrl, type Env } from "./config.js";
import { logError, logInfo, setLogLevel } from "./log.js";
import { sendBatchNotification } from "./slack.js";
import { extractOperations, type DetectedOperation } from "./squads/index.js";
import { parsePayload, validateAuth } from "./webhook.js";

export async function handleWebhook(
  request: Request,
  env: Env,
): Promise<Response> {
  let appConfig;
  try {
    appConfig = loadConfig(env);
    setLogLevel(appConfig.logLevel);
  } catch (e) {
    logError("config load failed", { error: String(e) });
    return new Response("internal error", { status: 500 });
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
    logError("payload parse failed", { error: String(e) });
    return new Response("bad request", { status: 400 });
  }

  const operations = await extractOperations(
    transactions,
    [...appConfig.multisigAddresses],
    appConfig.filter,
  );

  // Dedup by signature (Helius can include duplicates in one batch)
  const seen = new Set<string>();
  const deduped = operations.filter((op) => {
    if (seen.has(op.signature)) return false;
    seen.add(op.signature);
    return true;
  });

  logInfo("webhook processed", {
    transactions: transactions.length,
    operations: operations.length,
    deduped: deduped.length,
  });

  // Group operations by destination webhook URL
  const groups = new Map<string, DetectedOperation[]>();
  for (const op of deduped) {
    const url = resolveWebhookUrl(appConfig, op.multisig);
    const group = groups.get(url);
    if (group) {
      group.push(op);
    } else {
      groups.set(url, [op]);
    }
  }

  // Send one batched message per destination
  const results = await Promise.allSettled(
    [...groups.entries()].map(([url, ops]) =>
      sendBatchNotification(ops, url, appConfig.display),
    ),
  );
  for (const result of results) {
    if (result.status === "rejected") {
      logError("slack notification failed", { error: String(result.reason) });
    }
  }

  return new Response("ok");
}

export function handleHealth(env: Env): Response {
  try {
    loadConfig(env);
    return new Response(JSON.stringify({ status: "ok" }), {
      headers: { "Content-Type": "application/json" },
    });
  } catch (e) {
    return new Response(
      JSON.stringify({ status: "error", message: String(e) }),
      { status: 500, headers: { "Content-Type": "application/json" } },
    );
  }
}
