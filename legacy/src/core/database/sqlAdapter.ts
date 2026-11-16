/**
 * Minimal synchronous SQL adapter interface so the core repository runs cleanly
 * against Node's built-in `node:sqlite` (desktop, workers, tests) as well as
 * Capacitor SQLite or test doubles without duplicating repository logic.
 */

export type SqlParam = string | number | null | Uint8Array;

export interface SqlAdapter {
  exec(sql: string): void;
  run(sql: string, params?: SqlParam[]): void;
  get<T = Record<string, unknown>>(sql: string, params?: SqlParam[]): T | undefined;
  all<T = Record<string, unknown>>(sql: string, params?: SqlParam[]): T[];
  transaction<T>(fn: () => T): T;
  close(): void;
}
