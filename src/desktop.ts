import { invoke, isTauri } from "@tauri-apps/api/core";

export const desktop = isTauri();
export const call = <T>(command: string, args?: Record<string, unknown>) =>
  invoke<T>(command, args);
export const errorText = (error: unknown) =>
  error instanceof Error ? error.message : String(error);

export type Theme = "system" | "light" | "dark";
export type Project = { id: string; name: string; description: string };
export type Projects = { projects: Project[]; activeId: string | null };
export type AppInfo = {
  name: string;
  version: string;
  commit: string;
  builtAt: string;
  dataDir: string;
  logDir: string;
};
export type Mcp = { enabled: boolean; port: number; url: string | null };
export type Update = {
  status: "unconfigured" | "current" | "available";
  version?: string;
  notes?: string;
};
export type License = {
  state:
    | "unconfigured"
    | "unactivated"
    | "valid"
    | "expiring"
    | "expired"
    | "invalid"
    | "recovery";
  message: string;
  configured: boolean;
  expiresAt: string | null;
  warningAt: string | null;
  licenseId: string | null;
};
