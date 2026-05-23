import { RadWasm, createMemoryBackend } from '../wasm/loader';

export interface RelayOptions {
  port: string;
  storage: string;
  s3Endpoint?: string;
  s3Bucket?: string;
  s3AccessKey?: string;
  s3SecretKey?: string;
  s3Region?: string;
  wasm?: string;
}

export async function startRelay(opts: RelayOptions) {
  // 1. ストレージバックエンド選択
  let storage;

  if (opts.storage === 's3') {
    // Validate S3 options
    if (!opts.s3Endpoint || !opts.s3Bucket || !opts.s3AccessKey || !opts.s3SecretKey) {
      console.error('error: S3 storage requires --s3-endpoint, --s3-bucket, --s3-access-key, and --s3-secret-key');
      process.exit(1);
    }

    try {
      const { S3Backend } = await import('../storage/s3-backend');
      storage = new S3Backend({
        endpoint: opts.s3Endpoint,
        bucket: opts.s3Bucket,
        accessKey: opts.s3AccessKey,
        secretKey: opts.s3SecretKey,
        region: opts.s3Region || 'us-east-1',
      });
      console.log('rad relay using S3 storage: ' + opts.s3Endpoint + '/' + opts.s3Bucket);
    } catch (e) {
      console.error('error: Failed to initialize S3 storage:', (e as Error).message);
      process.exit(1);
    }
  } else {
    storage = createMemoryBackend();
  }

  // 2. WASM ロード
  const wasmPath = opts.wasm || './rad_wasm.wasm';
  const wasm = await RadWasm.load(wasmPath, storage);
  wasm.init();

  // 3. Hono アプリ作成
  const { createRelayApp } = await import('./app');
  const app = createRelayApp(wasm);

  // 4. サーバー起動
  const port = parseInt(opts.port);
  console.log('rad relay listening on port ' + port);

  Bun.serve({
    fetch: app.fetch,
    port,
  });
}
