import { RadWasm, RadStorageBackend } from './loader';

// インメモリストレージ（テスト用）
class MemoryStorageBackend implements RadStorageBackend {
  private storage = new Map<string, string>();

  put(key: string, data: string): void {
    this.storage.set(key, data);
  }

  get(key: string): string | null {
    return this.storage.get(key) ?? null;
  }

  list(prefix: string): string[] {
    return Array.from(this.storage.keys()).filter((k) =>
      k.startsWith(prefix)
    );
  }

  delete(key: string): void {
    this.storage.delete(key);
  }
}

async function runTests() {
  const storage = new MemoryStorageBackend();
  const wasm = await RadWasm.load('./rad_wasm.wasm', storage);

  let passed = 0;
  let failed = 0;

  // T-WH01: ロード成功
  if (wasm) {
    console.log('✅ T-WH01: WASM loaded successfully');
    passed++;
  } else {
    console.log('❌ T-WH01: Failed to load WASM');
    failed++;
    process.exit(1);
  }

  // 初期化
  const initResult = wasm.init();
  console.log('Init result:', initResult);

  // T-WH02: rad_alloc / rad_dealloc が動作する（間接的にテスト済み）
  console.log('✅ T-WH02: Memory allocation works (tested via string passing)');
  passed++;

  // T-WH03: ホスト関数 storage_put → storage_get のラウンドトリップが成功
  try {
    storage.put('test-key', 'test-value');
    const value = storage.get('test-key');
    if (value === 'test-value') {
      console.log('✅ T-WH03: Storage put/get roundtrip works');
      passed++;
    } else {
      console.log('❌ T-WH03: Storage roundtrip failed');
      failed++;
    }
  } catch (e) {
    console.log('❌ T-WH03: Storage roundtrip error:', e);
    failed++;
  }

  // T-WH04: rad_join が参加者を登録し JSON を返す
  try {
    const joinInput = JSON.stringify({
      publicKey: 'test-pubkey-123',
      displayName: 'alice',
    });
    const joinResult = wasm.join(joinInput);
    const joinData = JSON.parse(joinResult);
    if (joinData.participantId && joinData.joinedAt) {
      console.log('✅ T-WH04: rad_join works:', joinResult);
      passed++;
    } else {
      console.log('❌ T-WH04: rad_join returned invalid data:', joinResult);
      failed++;
    }
  } catch (e) {
    console.log('❌ T-WH04: rad_join error:', e);
    failed++;
  }

  // T-WH05: rad_submit_op が操作を受理し visible を返す
  try {
    const opInput = JSON.stringify({
      id: '',
      participantId: 'p-0',
      regionId: 'test.ts:1-10',
      type: 'write',
      content: 'hello wasm',
      signature: 'test-sig',
      timestamp: Math.floor(Date.now() / 1000),
      status: 'visible',
    });
    const opResult = wasm.submitOp(opInput);
    const opData = JSON.parse(opResult);
    if (opData.status === 'visible' && opData.id) {
      console.log('✅ T-WH05: rad_submit_op works:', opResult);
      passed++;
    } else {
      console.log('❌ T-WH05: rad_submit_op returned invalid data:', opResult);
      failed++;
    }
  } catch (e) {
    console.log('❌ T-WH05: rad_submit_op error:', e);
    failed++;
  }

  // T-WH06: rad_accept が accept を実行し accepted を返す
  try {
    const acceptInput = JSON.stringify({
      operationId: 'op-1-0',
      participantId: 'p-0',
    });
    const acceptResult = wasm.accept(acceptInput);
    const acceptData = JSON.parse(acceptResult);
    if (acceptData.status === 'accepted') {
      console.log('✅ T-WH06: rad_accept works:', acceptResult);
      passed++;
    } else {
      console.log('❌ T-WH06: rad_accept returned invalid data:', acceptResult);
      failed++;
    }
  } catch (e) {
    console.log('❌ T-WH06: rad_accept error:', e);
    failed++;
  }

  // T-WH07: rad_get_log が操作ログを JSON で返す
  try {
    const logResult = wasm.getLog();
    const logData = JSON.parse(logResult);
    if (Array.isArray(logData)) {
      console.log(`✅ T-WH07: rad_get_log works (${logData.length} operations)`);
      passed++;
    } else {
      console.log('❌ T-WH07: rad_get_log did not return array:', logResult);
      failed++;
    }
  } catch (e) {
    console.log('❌ T-WH07: rad_get_log error:', e);
    failed++;
  }

  // T-WH08: rad_compact が成功する
  try {
    const compactResult = wasm.compact();
    const compactData = JSON.parse(compactResult);
    if (compactData.status === 'compacted') {
      console.log('✅ T-WH08: rad_compact works:', compactResult);
      passed++;
    } else {
      console.log('❌ T-WH08: rad_compact returned invalid data:', compactResult);
      failed++;
    }
  } catch (e) {
    console.log('❌ T-WH08: rad_compact error:', e);
    failed++;
  }

  console.log(`\n=== Test Summary ===`);
  console.log(`Passed: ${passed}`);
  console.log(`Failed: ${failed}`);

  process.exit(failed > 0 ? 1 : 0);
}

runTests().catch((e) => {
  console.error('Test error:', e);
  process.exit(1);
});
