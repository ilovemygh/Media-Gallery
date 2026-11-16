import { DatabaseSync } from 'node:sqlite';
import { SCHEMA_SQL } from './schema.ts';
import type { SqlAdapter, SqlParam } from './sqlAdapter.ts';

/**
 * Node.js built-in SQLite adapter (`node:sqlite`) with WAL mode and FTS5 enabled.
 */
export class NodeSqliteAdapter implements SqlAdapter {
  private readonly db: DatabaseSync;
  private txDepth = 0;

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
    const isOuter = this.txDepth === 0;
    const spName = `sp_${this.txDepth}`;
    this.db.exec(isOuter ? 'BEGIN IMMEDIATE;' : `SAVEPOINT ${spName};`);
    this.txDepth += 1;
    try {
      const result = fn();
      this.txDepth -= 1;
      this.db.exec(isOuter ? 'COMMIT;' : `RELEASE SAVEPOINT ${spName};`);
      return result;
    } catch (error) {
      this.txDepth -= 1;
      this.db.exec(
        isOuter
          ? 'ROLLBACK;'
          : `ROLLBACK TO SAVEPOINT ${spName}; RELEASE SAVEPOINT ${spName};`,
      );
      throw error;
    }
  }

  close(): void {
    this.db.close();
  }
}
