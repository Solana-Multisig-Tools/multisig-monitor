import { handleHealth } from "../src/handler.js";

export const runtime = "edge";

export default function handler(): Response {
  return handleHealth(process.env as Record<string, string>);
}
