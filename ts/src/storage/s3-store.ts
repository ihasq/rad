import type { RadStorageBackend } from './backend';
import type { Operation, CodeRegion, Participant } from '../types';

/**
 * S3-backed RadStore implementation.
 * Uses RadStorageBackend abstraction for storage operations.
 *
 * S3 bucket structure:
 *   rad/
 *     config.json
 *     participants.json
 *     oplog/
 *       {timestamp}-{id}.json    ← individual operations
 *       _index.json              ← operation ID → key mapping
 *     regions.json
 *     founders.json
 *     snapshots/
 *       src/main.ts
 *       ...
 */
export class S3RadStore {
  constructor(private backend: RadStorageBackend) {}

  /**
   * Load all operations from S3
   */
  async loadOplog(): Promise<Operation[]> {
    const keys = await this.backend.list('rad/oplog/');
    const operations: Operation[] = [];

    for (const key of keys) {
      // Skip the index file
      if (key.endsWith('_index.json')) continue;

      const data = await this.backend.get(key);
      if (data) {
        try {
          operations.push(JSON.parse(data));
        } catch {
          // Skip corrupted entries
        }
      }
    }

    // Sort by timestamp
    operations.sort((a, b) => a.timestamp - b.timestamp);
    return operations;
  }

  /**
   * Save operations to S3
   * Each operation is stored as a separate object
   */
  async saveOplog(operations: Operation[]): Promise<void> {
    // Build index
    const index: Record<string, string> = {};

    for (const op of operations) {
      const key = `rad/oplog/${op.timestamp}-${op.id}.json`;
      await this.backend.put(key, JSON.stringify(op));
      index[op.id] = key;
    }

    // Save index
    await this.backend.put('rad/oplog/_index.json', JSON.stringify(index));
  }

  /**
   * Append a single operation to S3
   */
  async appendOp(op: Operation): Promise<void> {
    const key = `rad/oplog/${op.timestamp}-${op.id}.json`;
    await this.backend.put(key, JSON.stringify(op));

    // Update index
    const indexData = await this.backend.get('rad/oplog/_index.json');
    const index: Record<string, string> = indexData ? JSON.parse(indexData) : {};
    index[op.id] = key;
    await this.backend.put('rad/oplog/_index.json', JSON.stringify(index));
  }

  async loadRegions(): Promise<CodeRegion[]> {
    const data = await this.backend.get('rad/regions.json');
    if (!data) return [];

    try {
      return JSON.parse(data);
    } catch {
      return [];
    }
  }

  async saveRegions(regions: CodeRegion[]): Promise<void> {
    await this.backend.put('rad/regions.json', JSON.stringify(regions));
  }

  async loadParticipants(): Promise<Participant[]> {
    const data = await this.backend.get('rad/participants.json');
    if (!data) return [];

    try {
      return JSON.parse(data);
    } catch {
      return [];
    }
  }

  async saveParticipants(participants: Participant[]): Promise<void> {
    await this.backend.put('rad/participants.json', JSON.stringify(participants));
  }

  async loadFounders(): Promise<{ founders: Record<string, string>; rootFounder: string }> {
    // Load root founder from config
    let rootFounder = '';
    const configData = await this.backend.get('rad/config.json');
    if (configData) {
      try {
        const config = JSON.parse(configData);
        rootFounder = config.founder || '';
      } catch {}
    }

    // Load founders map
    const foundersData = await this.backend.get('rad/founders.json');
    if (foundersData) {
      try {
        const founders = JSON.parse(foundersData);
        return { founders, rootFounder };
      } catch {}
    }

    return { founders: { '.': rootFounder }, rootFounder };
  }

  async saveFounders(founders: Record<string, string>): Promise<void> {
    await this.backend.put('rad/founders.json', JSON.stringify(founders));
  }

  async putSnapshot(filePath: string, content: string): Promise<void> {
    const key = `rad/snapshots/${filePath}`;
    await this.backend.put(key, content);
  }

  async getSnapshot(filePath: string): Promise<string | null> {
    return await this.backend.get(`rad/snapshots/${filePath}`);
  }

  /**
   * Compact operation log
   * Moves accepted writes to snapshots and removes them from oplog
   */
  async compact(): Promise<void> {
    const operations = await this.loadOplog();

    // Group operations by file path and collect accepted writes
    const fileContents = new Map<string, string>();
    const acceptedIds = new Set<string>();

    for (const op of operations) {
      if (op.status === 'accepted' && op.type === 'write') {
        // Extract file path from regionId (format: "file:start-end")
        const colonPos = op.regionId.indexOf(':');
        if (colonPos !== -1) {
          const filePath = op.regionId.substring(0, colonPos);
          fileContents.set(filePath, op.content);
          acceptedIds.add(op.id);
        }
      }
    }

    // Write snapshots
    for (const [filePath, content] of fileContents.entries()) {
      await this.putSnapshot(filePath, content);
    }

    // Remove accepted operations from S3
    for (const op of operations) {
      if (acceptedIds.has(op.id)) {
        const key = `rad/oplog/${op.timestamp}-${op.id}.json`;
        await this.backend.delete(key);
      }
    }

    // Update index to remove deleted operations
    const indexData = await this.backend.get('rad/oplog/_index.json');
    if (indexData) {
      const index: Record<string, string> = JSON.parse(indexData);
      for (const id of acceptedIds) {
        delete index[id];
      }
      await this.backend.put('rad/oplog/_index.json', JSON.stringify(index));
    }
  }
}
