import { RadWasm, RadStorageBackend } from './loader';

class MemoryStorageBackend implements RadStorageBackend {
  private storage = new Map<string, string>();

  put(key: string, data: string): void {
    console.log(`Storage PUT: ${key} = ${data.substring(0, 100)}`);
    this.storage.set(key, data);
  }

  get(key: string): string | null {
    const result = this.storage.get(key) ?? null;
    console.log(`Storage GET: ${key} = ${result?.substring(0, 100)}`);
    return result;
  }

  list(prefix: string): string[] {
    const result = Array.from(this.storage.keys()).filter((k) =>
      k.startsWith(prefix)
    );
    console.log(`Storage LIST: ${prefix} = ${result}`);
    return result;
  }

  delete(key: string): void {
    console.log(`Storage DELETE: ${key}`);
    this.storage.delete(key);
  }
}

async function debugTest() {
  const storage = new MemoryStorageBackend();
  const wasm = await RadWasm.load('./rad_wasm.wasm', storage);

  console.log('=== Testing rad_init ===');
  const initReturnCode = (wasm as any).instance.exports.rad_init();
  console.log('Init return code:', initReturnCode);
  const initResult = wasm.init();
  console.log('Init result:', JSON.stringify(initResult));
  console.log('Init result length:', initResult.length);
  console.log('Init result bytes:', [...initResult].map(c => c.charCodeAt(0)));

  console.log('\n=== Testing rad_join ===');
  const joinInput = JSON.stringify({
    publicKey: 'test-pubkey-123',
    displayName: 'alice',
  });
  console.log('Join input:', joinInput);
  const joinResult = wasm.join(joinInput);
  console.log('Join result:', JSON.stringify(joinResult));
  console.log('Join result length:', joinResult.length);

  if (joinResult.trim()) {
    try {
      const joinData = JSON.parse(joinResult);
      console.log('Join data parsed:', joinData);
    } catch (e) {
      console.log('Join parse error:', e);
    }
  }
}

debugTest().catch((e) => {
  console.error('Debug test error:', e);
  process.exit(1);
});
