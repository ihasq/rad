import { Hono } from 'hono';
import type { RadWasm } from '../wasm/loader';
import {
  wasmToHttpStatus,
  toJoinResponse,
  toSubmitOpResponse,
  toAcceptResponse,
  toOpStatusResponse,
  toErrorResponse,
  type WasmResult
} from './transform';
import { generateOpId, currentTimestamp } from './idgen';

export function createRelayApp(wasm: RadWasm) {
  const app = new Hono();

  // 参加者登録
  app.post('/rad/participants', async (c) => {
    const body = await c.req.text();
    const raw = wasm.join(body);
    const result: WasmResult = JSON.parse(raw);
    if (!result.ok) {
      return c.json(toErrorResponse(result), wasmToHttpStatus(result));
    }
    return c.json(toJoinResponse(result.data), 201);
  });

  app.get('/rad/participants', (c) => {
    const raw = wasm.getParticipants();
    const result: WasmResult = JSON.parse(raw);
    return c.json(result.ok ? result.data : [], 200);
  });

  // 操作送信
  app.post('/rad/operations', async (c) => {
    const body = JSON.parse(await c.req.text());

    // === Relay の責務: ID, timestamp, status, reason を注入 ===
    body.id = generateOpId();
    body.timestamp = currentTimestamp();
    body.status = 'visible';
    if (body.reason === undefined) body.reason = null;

    const raw = wasm.submitOp(JSON.stringify(body));
    const result: WasmResult = JSON.parse(raw);
    if (!result.ok) {
      return c.json(toErrorResponse(result), wasmToHttpStatus(result));
    }
    // === Relay の責務: 内部スキーマ → OpenAPI スキーマ ===
    return c.json(toSubmitOpResponse(result.data), 201);
  });

  // accept
  app.post('/rad/accept', async (c) => {
    const body = await c.req.text();
    const raw = wasm.accept(body);
    const result: WasmResult = JSON.parse(raw);
    if (!result.ok) {
      return c.json(toErrorResponse(result), wasmToHttpStatus(result));
    }
    return c.json(toAcceptResponse(result.data), 200);
  });

  // 読み取り
  app.get('/rad/operations/:id/status', (c) => {
    const raw = wasm.getOpStatus(c.req.param('id'));
    const result: WasmResult = JSON.parse(raw);
    if (!result.ok) {
      return c.json(toErrorResponse(result), wasmToHttpStatus(result));
    }
    return c.json(toOpStatusResponse(result.data), 200);
  });

  app.get('/rad/operations/:id', (c) => {
    const raw = wasm.getOp(c.req.param('id'));
    const result: WasmResult = JSON.parse(raw);
    if (!result.ok) return c.json(toErrorResponse(result), 404);
    return c.json(result.data, 200);
  });

  app.get('/rad/visible/:path{.+}', (c) => {
    const raw = wasm.getVisible(c.req.param('path'));
    const result: WasmResult = JSON.parse(raw);
    return c.json(result.ok ? result.data : [], 200);
  });

  app.get('/rad/files/:path{.+}', (c) => {
    const raw = wasm.getFile(c.req.param('path'));
    const result: WasmResult = JSON.parse(raw);
    if (!result.ok) return c.json(toErrorResponse(result), 404);
    return c.json(result.data, 200);
  });

  app.get('/rad/files', (c) => {
    const raw = wasm.getFileList();
    const result: WasmResult = JSON.parse(raw);
    return c.json(result.ok ? result.data : [], 200);
  });

  app.get('/rad/regions/:path{.+}', (c) => {
    const raw = wasm.getRegions(c.req.param('path'));
    const result: WasmResult = JSON.parse(raw);
    return c.json(result.ok ? result.data : [], 200);
  });

  app.get('/rad/log', (c) => {
    const query = JSON.stringify({
      regionId: c.req.query('regionId') ?? null,
      participantId: c.req.query('participantId') ?? null,
      since: c.req.query('since') ? parseInt(c.req.query('since')!) : null,
      limit: c.req.query('limit') ? parseInt(c.req.query('limit')!) : null,
    });
    const raw = wasm.getLog(query);
    const result: WasmResult = JSON.parse(raw);
    return c.json(result.ok ? result.data : [], 200);
  });

  app.post('/rad/compact', (c) => {
    const raw = wasm.compact();
    const result: WasmResult = JSON.parse(raw);
    if (!result.ok) return c.json(toErrorResponse(result), 500);
    return c.json(result.data, 200);
  });

  app.post('/rad/sync/git', (c) => c.json({ error: 'Not implemented' }, 501));

  return app;
}
