import type { Env } from "./config.js";
import { handleWebhook } from "./handler.js";

/**
 * Cloudflare Workers entry point.
 * Wrangler passes the env bindings (vars + secrets) as the second argument.
 */
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
