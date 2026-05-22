import { Hono } from 'hono';
import type { RelayState } from '../app';

export function createReadRoutes(state: RelayState) {
  const app = new Hono();

  // GET /rad/visible/:filePath - visible な write 一覧
  app.get('/rad/visible/:filePath', (c) => {
    const filePath = c.req.param('filePath');
    const ops = state.oplog.getAllOperations();

    // filePath に関連する visible な write 操作を抽出
    const visibleWrites = ops.filter(op =>
      op.status === 'visible' &&
      op.type === 'write' &&
      op.regionId.startsWith(filePath + ':')
    );

    return c.json(visibleWrites, 200);
  });

  // GET /rad/files/:filePath - accepted なファイル内容
  app.get('/rad/files/:filePath', (c) => {
    const filePath = c.req.param('filePath');
    const ops = state.oplog.getAllOperations();

    // filePath に関連する accepted な write 操作を抽出
    const acceptedWrites = ops.filter(op =>
      op.status === 'accepted' &&
      op.type === 'write' &&
      op.regionId.startsWith(filePath + ':')
    );

    // 最新の内容を返す（最後の accepted write）
    if (acceptedWrites.length > 0) {
      const latest = acceptedWrites[acceptedWrites.length - 1];
      return c.text(latest.content, 200);
    }

    return c.json({ error: 'file not found or no accepted writes' }, 404);
  });

  // GET /rad/files - ファイル一覧
  app.get('/rad/files', (c) => {
    const ops = state.oplog.getAllOperations();
    const files = new Set<string>();

    ops.forEach(op => {
      if (op.type === 'write') {
        const filePath = op.regionId.split(':')[0];
        files.add(filePath);
      }
    });

    return c.json(Array.from(files), 200);
  });

  // GET /rad/regions/:filePath - コード領域一覧
  app.get('/rad/regions/:filePath', (c) => {
    const filePath = c.req.param('filePath');
    const regions = state.regionMap.getAllRegions();

    // filePath に関連する領域を抽出
    const fileRegions = regions.filter(r => r.filePath === filePath);

    return c.json(fileRegions, 200);
  });

  // GET /rad/log - 操作ログ
  app.get('/rad/log', (c) => {
    const regionId = c.req.query('regionId');
    const participantId = c.req.query('participantId');
    const since = c.req.query('since');
    const limit = c.req.query('limit');

    let ops = state.oplog.getAllOperations();

    // フィルタリング
    if (regionId) {
      ops = ops.filter(op => op.regionId === regionId);
    }
    if (participantId) {
      ops = ops.filter(op => op.participantId === participantId);
    }
    if (since) {
      const sinceTs = parseInt(since);
      ops = ops.filter(op => op.timestamp >= sinceTs);
    }
    if (limit) {
      const limitNum = parseInt(limit);
      ops = ops.slice(0, limitNum);
    }

    return c.json(ops, 200);
  });

  // POST /rad/compact - コンパクション実行
  app.post('/rad/compact', (c) => {
    // RP9 では簡易実装（accepted 操作を削除するのみ）
    // RP12 で永続化と合わせて完全実装
    const ops = state.oplog.getAllOperations();
    const acceptedOps = ops.filter(op => op.status === 'accepted');
    const nonAcceptedOps = ops.filter(op => op.status !== 'accepted');

    // accepted でない操作のみを残す
    state.oplog.loadOperations(nonAcceptedOps);

    return c.json({
      compacted: acceptedOps.length,
      message: 'compaction completed'
    }, 200);
  });

  // POST /rad/sync/git - Git 双方向同期（RP11 で実装）
  app.post('/rad/sync/git', (c) => {
    return c.json({ error: 'not implemented yet (RP11)' }, 501);
  });

  return app;
}
