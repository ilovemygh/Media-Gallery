import { DatabaseSync } from 'node:sqlite';
import { SCHEMA_SQL } from './schema.ts';
import type { SqlAdapter, SqlParam } from './sqlAdapter.ts';

/**
 * Node.js built-in SQLite adapter (`node:sqlite`) with WAL mode and FTS5 enabled.
 */
export class NodeSqliteAdapter implements SqlAdapter {
  private readonly db: DatabaseSync;

  constructor(dbPath: string = ':memory:') {
    this.db = new DatabaseSync(dbPath);
    if (dbPath !== ':memory:') {
      this.db.exec('PRAGMA journal_mode = WAL;');
    }
    this.db.exec('PRAGMA synchronous = NORMAL;');
    this.db.exec(SCHEMA_SQL);
  }

  exec(sql: string): void {
    this.db.exec(sql);
  }

  run(sql: string, params: SqlParam[] = []): void {
    const stmt = this.db.prepare(sql);
    stmt.run(...params);
  }

  get<T = Record<string, unknown>>(sql: string, params: SqlParam[] = []): T | undefined {
    const stmt = this.db.prepare(sql);
    return stmt.get(...params) as T | undefined;
  }

  all<T = Record<string, unknown>>(sql: string, params: SqlParam[] = []): T[] {
    const stmt = this.db.prepare(sql);
    return stmt.all(...params) as T[];
  }

  transaction<T>(fn: () => T): T {
    this.db.exec('BEGIN IMMEDIATE;');
    try {
      const result = fn();
      this.db.exec('COMMIT;');
      return result;
    } catch (error) {
      this.db.exec('ROLLBACK;');
      throw error;
    }
  }

  close(): void {
    this.db.close();
  }
}
