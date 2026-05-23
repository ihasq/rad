import { Hono } from 'hono';
import type { RadWasm } from '../wasm/loader';

export function createRelayApp(wasm: RadWasm) {
  const app = new Hono();

  // 参加者
  app.post('/rad/participants', async (c) => {
    const body = await c.req.text();
    const result = wasm.join(body);
    const parsed = JSON.parse(result);
    if (parsed.error) return c.json(parsed, 400);
    return c.json(parsed, 201);
  });

  app.get('/rad/participants', (c) => {
    const result = wasm.getParticipants();
    return c.json(JSON.parse(result));
  });

  // 操作
  app.post('/rad/operations', async (c) => {
    const body = await c.req.text();
    const result = wasm.submitOp(body);
    const parsed = JSON.parse(result);
    if (parsed.error) {
      const status = parsed.code === 'INVALID_SIGNATURE' ? 403 : 400;
      return c.json(parsed, status);
    }
    return c.json(parsed, 201);
  });

  // accept
  app.post('/rad/accept', async (c) => {
    const body = await c.req.text();
    const result = wasm.accept(body);
    const parsed = JSON.parse(result);
    if (parsed.error) {
      const status = parsed.code === 'NOT_LEADER' ? 403 : 409;
      return c.json(parsed, status);
    }
    return c.json(parsed, 200);
  });

  // 読み取り
  app.get('/rad/operations/:id/status', (c) => {
    const result = wasm.getOpStatus(c.req.param('id'));
    return c.json(JSON.parse(result));
  });

  app.get('/rad/operations/:id', (c) => {
    const result = wasm.getOp(c.req.param('id'));
    const parsed = JSON.parse(result);
    if (parsed.error) return c.json(parsed, 404);
    return c.json(parsed);
  });

  app.get('/rad/visible/:path{.+}', (c) => {
    const result = wasm.getVisible(c.req.param('path'));
    return c.json(JSON.parse(result));
  });

  app.get('/rad/files/:path{.+}', (c) => {
    const result = wasm.getFile(c.req.param('path'));
    const parsed = JSON.parse(result);
    if (parsed.error) return c.json(parsed, 404);
    return c.json(parsed);
  });

  app.get('/rad/files', (c) => {
    const result = wasm.getFileList();
    return c.json(JSON.parse(result));
  });

  app.get('/rad/regions/:path{.+}', (c) => {
    const result = wasm.getRegions(c.req.param('path'));
    return c.json(JSON.parse(result));
  });

  app.get('/rad/log', (c) => {
    const query = JSON.stringify({
      regionId: c.req.query('regionId'),
      participantId: c.req.query('participantId'),
      since: c.req.query('since'),
      limit: c.req.query('limit'),
    });
    const result = wasm.getLog(query);
    return c.json(JSON.parse(result));
  });

  app.post('/rad/compact', (c) => {
    const result = wasm.compact();
    return c.json(JSON.parse(result));
  });

  app.post('/rad/sync/git', (c) => c.json({ error: 'Not implemented' }, 501));

  return app;
}
