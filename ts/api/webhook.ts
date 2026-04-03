/**
 * Vercel Edge Function entry point.
 * Reads config from process.env and delegates to the shared webhook handler.
 */
import { handleWebhook } from "../src/handler.js";

export const config = { runtime: "edge" };

export default async function (request: Request): Promise<Response> {
  return handleWebhook(request, process.env as Record<string, string>);
}
