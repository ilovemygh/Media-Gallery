import { randomUUID } from 'node:crypto';
import type { MediaRepository } from '../../core/database/repository.ts';
import type {
  MediaSearchFilters,
  MediaWithCover,
  SavedSearchRecord,
} from '../../core/types/domain.ts';

/**
 * Instant FTS5 search, faceted filtering, and smart-folder saved searches (PRD C1, C5, C6).
 */
export class SearchService {
  constructor(private readonly repo: MediaRepository) {}

  search(queryText: string, filters: MediaSearchFilters = {}): {
    items: MediaWithCover[];
    totalCount: number;
  } {
    const items = this.repo.queryMedia(queryText, filters);
    const totalCount = this.repo.countMedia(queryText, filters);
    return { items, totalCount };
  }

  saveSearch(name: string, query: string, filters: MediaSearchFilters): SavedSearchRecord {
    const record: SavedSearchRecord = {
      id: randomUUID(),
      name: name.trim(),
      query,
      filters,
      createdAt: Date.now(),
    };
    this.repo.upsertSavedSearch(record);
    return record;
  }

  listSavedSearches(): SavedSearchRecord[] {
    return this.repo.listSavedSearches();
  }

  deleteSavedSearch(id: string): void {
    this.repo.deleteSavedSearch(id);
  }
}
