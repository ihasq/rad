/**
 * Storage backend abstraction for Rad.
 * Allows pluggable storage implementations (filesystem, S3, etc.)
 */
export interface RadStorageBackend {
  /**
   * Store data at the given key
   */
  put(key: string, data: string): Promise<void>;

  /**
   * Retrieve data from the given key
   * Returns null if key does not exist
   */
  get(key: string): Promise<string | null>;

  /**
   * List all keys with the given prefix
   */
  list(prefix: string): Promise<string[]>;

  /**
   * Delete the object at the given key
   */
  delete(key: string): Promise<void>;
}
