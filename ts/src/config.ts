import { NotificationFilter, parseCommaList, validateFilter } from "./filter.js";
import { type LogLevel, parseLogLevel } from "./log.js";

export interface DisplayConfig {
  readonly showPermissions: boolean;
  readonly showTimestamps: boolean;
  readonly truncateAddresses: boolean;
}

export interface Config {
  readonly multisigAddresses: readonly string[];
  readonly heliusAuthToken: string;
  readonly slackWebhookUrl: string;
  readonly filter: NotificationFilter;
  readonly display: DisplayConfig;
  readonly logLevel: LogLevel;
  readonly routes: ReadonlyMap<string, string>;
}

export interface Env {
  readonly MULTISIG_ADDRESSES?: string;
  readonly HELIUS_AUTH_TOKEN?: string;
  readonly SLACK_WEBHOOK_URL?: string;
  readonly INSTRUCTION_TYPES?: string;
  readonly CONFIG_TYPES?: string;
  readonly SHOW_PERMISSIONS?: string;
  readonly SHOW_TIMESTAMPS?: string;
  readonly TRUNCATE_ADDRESSES?: string;
  readonly LOG_LEVEL?: string;
  readonly MULTISIG_ROUTES?: string;
}

function parseBool(value: string | undefined): boolean {
  return value?.toLowerCase() === "true";
}

function parseRoutes(raw: string): Map<string, string> {
  const routes = new Map<string, string>();
  if (!raw) return routes;
  for (const entry of raw.split(",")) {
    const sep = entry.indexOf(":");
    if (sep === -1) continue;
    const addr = entry.slice(0, sep).trim();
    const url = entry.slice(sep + 1).trim();
    if (addr && url) routes.set(addr, url);
  }
  return routes;
}

export function resolveWebhookUrl(config: Config, multisig: string): string {
  return config.routes.get(multisig) ?? config.slackWebhookUrl;
}

export function loadConfig(env: Env): Config {
  const multisigAddresses = parseCommaList(env.MULTISIG_ADDRESSES ?? "");
  if (multisigAddresses.length === 0) {
    throw new Error("MULTISIG_ADDRESSES is required");
  }

  const heliusAuthToken = env.HELIUS_AUTH_TOKEN;
  if (!heliusAuthToken) {
    throw new Error("HELIUS_AUTH_TOKEN is required");
  }

  const slackWebhookUrl = env.SLACK_WEBHOOK_URL;
  if (!slackWebhookUrl) {
    throw new Error("SLACK_WEBHOOK_URL is required");
  }

  const filter: NotificationFilter = {
    instructionTypes: parseCommaList(env.INSTRUCTION_TYPES ?? ""),
    configTypes: parseCommaList(env.CONFIG_TYPES ?? ""),
  };
  validateFilter(filter);

  const display: DisplayConfig = {
    showPermissions: parseBool(env.SHOW_PERMISSIONS),
    showTimestamps: parseBool(env.SHOW_TIMESTAMPS),
    truncateAddresses: parseBool(env.TRUNCATE_ADDRESSES),
  };

  const logLevel = parseLogLevel(env.LOG_LEVEL);

  const routes = parseRoutes(env.MULTISIG_ROUTES ?? "");

  return { multisigAddresses, heliusAuthToken, slackWebhookUrl, filter, display, logLevel, routes };
}
