import { AwsClient } from 'aws4fetch';
import type { RadStorageBackend } from './backend';

export interface S3Config {
  endpoint: string;
  bucket: string;
  accessKey: string;
  secretKey: string;
  region: string;
}

/**
 * S3-compatible storage backend.
 * Works with AWS S3, Cloudflare R2, Backblaze B2, iDrive, MinIO, etc.
 */
export class S3Backend implements RadStorageBackend {
  private client: AwsClient;
  private endpoint: string;
  private bucket: string;
  private bucketCreated: boolean = false;

  constructor(config: S3Config) {
    this.client = new AwsClient({
      accessKeyId: config.accessKey,
      secretAccessKey: config.secretKey,
      region: config.region,
      service: 's3',
    });
    this.endpoint = config.endpoint;
    this.bucket = config.bucket;
  }

  private async ensureBucket(): Promise<void> {
    if (this.bucketCreated) {
      return;
    }

    try {
      // Try to create the bucket (PUT with empty body)
      const url = `${this.endpoint}/${this.bucket}`;
      const res = await this.client.fetch(url, {
        method: 'PUT',
        body: '',
      });

      // Bucket creation succeeds with 200, 201, or 409 (already exists)
      if (res.ok || res.status === 409) {
        this.bucketCreated = true;
        return;
      }

      // If bucket creation failed, try a test PUT to see if bucket exists
      const testKey = `_test/${Date.now()}`;
      const testUrl = `${this.endpoint}/${this.bucket}/${testKey}`;
      const testRes = await this.client.fetch(testUrl, {
        method: 'PUT',
        body: 'test',
      });

      if (testRes.ok) {
        this.bucketCreated = true;
        // Clean up test file
        await this.client.fetch(testUrl, { method: 'DELETE' });
      }
    } catch (e) {
      // Bucket creation might have failed, but bucket might exist
      console.warn('Bucket initialization warning:', e);
    }
  }

  async put(key: string, data: string): Promise<void> {
    await this.ensureBucket();

    const url = `${this.endpoint}/${this.bucket}/${key}`;
    const res = await this.client.fetch(url, {
      method: 'PUT',
      body: data,
    });

    if (!res.ok) {
      const errorBody = await res.text();
      console.error(`S3 PUT failed for ${key}:`, res.status, res.statusText, errorBody.substring(0, 200));
      throw new Error(`S3 PUT failed: ${res.status} ${res.statusText}`);
    }
  }

  async get(key: string): Promise<string | null> {
    const url = `${this.endpoint}/${this.bucket}/${key}`;
    const res = await this.client.fetch(url);

    if (res.status === 404) {
      return null;
    }

    if (!res.ok) {
      // Handle bucket not found or other errors during initialization
      console.warn(`S3 GET warning for ${key}: ${res.status} ${res.statusText}`);
      return null;
    }

    return await res.text();
  }

  async list(prefix: string): Promise<string[]> {
    const url = `${this.endpoint}/${this.bucket}?list-type=2&prefix=${encodeURIComponent(prefix)}`;
    const res = await this.client.fetch(url);

    // Handle bucket not found or no objects - return empty array
    if (res.status === 404) {
      return [];
    }

    if (!res.ok) {
      // For other errors, log but return empty to allow initialization
      console.warn(`S3 LIST warning: ${res.status} ${res.statusText}`);
      return [];
    }

    const xml = await res.text();

    // Parse XML to extract keys
    // S3 ListObjectsV2 response format:
    // <ListBucketResult><Contents><Key>path/to/file</Key></Contents>...</ListBucketResult>
    const keys: string[] = [];
    const keyRegex = /<Key>([^<]+)<\/Key>/g;
    let match;

    while ((match = keyRegex.exec(xml)) !== null) {
      keys.push(match[1]);
    }

    return keys.sort();
  }

  async delete(key: string): Promise<void> {
    const url = `${this.endpoint}/${this.bucket}/${key}`;
    const res = await this.client.fetch(url, {
      method: 'DELETE',
    });

    if (!res.ok && res.status !== 404) {
      throw new Error(`S3 DELETE failed: ${res.status} ${res.statusText}`);
    }
  }
}
