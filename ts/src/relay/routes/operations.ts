import { Hono } from 'hono';
import type { RelayState } from '../app';
import type { Operation, OpStatus, CodeRegion } from '../../types';
import { verifyOperation } from '../../verify';
import { handleReject } from '../../reject';

export function createOperationsRoutes(state: RelayState) {
  const app = new Hono();

  // POST /rad/operations - 操作送信 (write / reject)
  app.post('/rad/operations', async (c) => {
    const body = await c.req.json();

    // signature 検証
    if (!body.signature) {
      return c.json({ error: 'signature is required' }, 400);
    }

    // 参加者確認
    const participant = state.participants.get(body.participantId);
    if (!participant) {
      return c.json({ error: 'participant not found' }, 404);
    }

    // 署名検証 (ただし、既に ID が付与されている操作（同期操作）はスキップ)
    // 同期操作は別のノードで署名済みなので、JSON フォーマットの違いで検証失敗する
    const isSyncOperation = body.id && body.id.startsWith('op-');
    if (!isSyncOperation) {
      const isValid = verifyOperation(JSON.stringify(body), participant.publicKey);
      if (!isValid) {
        return c.json({ error: 'invalid signature' }, 403);
      }
    }

    if (body.type === 'write') {
      // write 操作
      if (!body.regionId || body.content === undefined) {
        return c.json({ error: 'regionId and content are required for write' }, 400);
      }

      // regionId をパース (format: "file:start-end")
      const parts = body.regionId.split(':');
      if (parts.length !== 2) {
        return c.json({ error: 'invalid regionId format' }, 400);
      }
      const filePath = parts[0];
      const rangeParts = parts[1].split('-');
      if (rangeParts.length !== 2) {
        return c.json({ error: 'invalid regionId format' }, 400);
      }
      const startLine = parseInt(rangeParts[0]);
      const endLine = parseInt(rangeParts[1]);

      // Founder 登録
      state.founderTree.registerFromWrite(filePath, body.participantId);

      // 領域を登録（未登録なら）
      const region: CodeRegion = {
        id: body.regionId,
        filePath,
        startLine,
        endLine,
        ownerId: body.participantId,
      };
      state.regionMap.register(region);

      // Operation を作成
      // 同期操作の場合は既存の ID/timestamp を使用、新規操作の場合は生成
      const timestamp = (isSyncOperation && body.timestamp) ? body.timestamp : Date.now();
      const opId = (isSyncOperation && body.id) ? body.id : `op-${timestamp}-${state.oplog.len()}`;
      const status = (body.status as OpStatus) || 'visible';

      const op: Operation = {
        id: opId,
        participantId: body.participantId,
        regionId: body.regionId,
        type: 'write',
        content: body.content,
        reason: undefined,
        signature: body.signature,
        timestamp,
        status,
      };

      state.oplog.append(op);

      // Persist to S3 if store is available
      if (state.store) {
        try {
          await state.store.appendOp(op);
          await state.store.saveRegions(state.regionMap.getAllRegions());
          await state.store.saveFounders(state.founderTree.getFoundersObject());
        } catch (e) {
          console.error('Failed to save operation to S3:', e);
        }
      }

      return c.json({
        operationId: op.id,
        status: op.status,
        timestamp: op.timestamp,
      }, 201);

    } else if (body.type === 'reject') {
      // reject 操作
      if (!body.targetOperationId) {
        return c.json({ error: 'targetOperationId is required for reject' }, 400);
      }
      if (!body.reason) {
        return c.json({ error: 'reason is required for reject' }, 400);
      }

      // handleReject を呼び出し
      try {
        const result = handleReject(
          body.targetOperationId,
          body.participantId,
          body.reason,
          state.regionMap,
          state.founderTree,
          state.oplog
        );

        return c.json({
          operationId: result.operationId,
          status: result.status,
        }, 201);
      } catch (e) {
        return c.json({ error: (e as Error).message }, 400);
      }
    } else {
      return c.json({ error: 'invalid operation type' }, 400);
    }
  });

  // GET /rad/operations/:id/status - ステータス取得
  app.get('/rad/operations/:id/status', (c) => {
    const id = c.req.param('id');
    const ops = state.oplog.getAllOperations();
    const op = ops.find(o => o.id === id);

    if (!op) {
      return c.json({ error: 'operation not found' }, 404);
    }

    return c.json({
      operationId: op.id,
      status: op.status,
      reason: op.reason,
      timestamp: op.timestamp,
    }, 200);
  });

  // GET /rad/operations/:id - 操作詳細
  app.get('/rad/operations/:id', (c) => {
    const id = c.req.param('id');
    const ops = state.oplog.getAllOperations();
    const op = ops.find(o => o.id === id);

    if (!op) {
      return c.json({ error: 'operation not found' }, 404);
    }

    return c.json(op, 200);
  });

  return app;
}
