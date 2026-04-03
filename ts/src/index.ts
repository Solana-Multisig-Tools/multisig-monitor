import type { Env } from "./config.js";
import { handleHealth, handleWebhook } from "./handler.js";

/**
 * Cloudflare Workers entry point.
 * Wrangler passes the env bindings (vars + secrets) as the second argument.
 */
export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);

    if (request.method === "GET" && url.pathname === "/health") {
      return handleHealth(env);
    }

    if (request.method === "POST" && url.pathname === "/webhook") {
      return handleWebhook(request, env);
    }

    return new Response("not found", { status: 404 });
  },
};
