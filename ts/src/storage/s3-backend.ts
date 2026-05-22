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

  constructor(config: S3Config) {
    this.client = new AwsClient({
      accessKeyId: config.accessKey,
      secretAccessKey: config.secretKey,
      region: config.region,
    });
    this.endpoint = config.endpoint;
    this.bucket = config.bucket;
  }

  async put(key: string, data: string): Promise<void> {
    const url = `${this.endpoint}/${this.bucket}/${key}`;
    const res = await this.client.fetch(url, {
      method: 'PUT',
      body: data,
    });

    if (!res.ok) {
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
      throw new Error(`S3 GET failed: ${res.status} ${res.statusText}`);
    }

    return await res.text();
  }

  async list(prefix: string): Promise<string[]> {
    const url = `${this.endpoint}/${this.bucket}?list-type=2&prefix=${encodeURIComponent(prefix)}`;
    const res = await this.client.fetch(url);

    if (!res.ok) {
      throw new Error(`S3 LIST failed: ${res.status} ${res.statusText}`);
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
