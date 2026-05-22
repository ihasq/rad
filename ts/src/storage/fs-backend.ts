import { readFileSync, writeFileSync, mkdirSync, readdirSync, unlinkSync, existsSync, statSync } from 'node:fs';
import { join, dirname, relative } from 'node:path';
import type { RadStorageBackend } from './backend';

/**
 * Filesystem-based storage backend.
 * Stores data in a directory structure (e.g., .rad/)
 */
export class FileSystemBackend implements RadStorageBackend {
  constructor(private radDir: string) {}

  async put(key: string, data: string): Promise<void> {
    const path = join(this.radDir, key);
    mkdirSync(dirname(path), { recursive: true });
    writeFileSync(path, data, 'utf-8');
  }

  async get(key: string): Promise<string | null> {
    try {
      const path = join(this.radDir, key);
      return readFileSync(path, 'utf-8');
    } catch {
      return null;
    }
  }

  async list(prefix: string): Promise<string[]> {
    const prefixPath = join(this.radDir, prefix);
    const results: string[] = [];

    const walk = (dir: string) => {
      if (!existsSync(dir)) return;

      const entries = readdirSync(dir);
      for (const entry of entries) {
        const fullPath = join(dir, entry);
        const stat = statSync(fullPath);

        if (stat.isDirectory()) {
          walk(fullPath);
        } else {
          // Convert absolute path to relative key
          const relPath = relative(this.radDir, fullPath);
          if (relPath.startsWith(prefix)) {
            results.push(relPath);
          }
        }
      }
    };

    // Walk from the prefix directory if it exists
    const baseDir = dirname(prefixPath);
    walk(baseDir);

    return results.sort();
  }

  async delete(key: string): Promise<void> {
    const path = join(this.radDir, key);
    try {
      unlinkSync(path);
    } catch {
      // Ignore errors if file doesn't exist
    }
  }
}
