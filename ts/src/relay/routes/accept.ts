import { Hono } from 'hono';
import type { RelayState } from '../app';
import { verifyOperation } from '../../verify';
import { handleAccept } from '../../accept';

export function createAcceptRoutes(state: RelayState) {
  const app = new Hono();

  // POST /rad/accept - Leader が accept
  app.post('/rad/accept', async (c) => {
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

    // 署名検証
    const isValid = verifyOperation(JSON.stringify(body), participant.publicKey);
    if (!isValid) {
      return c.json({ error: 'invalid signature' }, 403);
    }

    // operationId 検証
    if (!body.operationId) {
      return c.json({ error: 'operationId is required' }, 400);
    }

    // handleAccept を呼び出し
    try {
      const result = handleAccept(
        body.operationId,
        body.participantId,
        state.regionMap,
        state.oplog
      );

      return c.json({
        operationId: result.operationId,
        status: result.status,
      }, 200);
    } catch (e) {
      const errorMsg = (e as Error).message;
      // Leader でない場合は 403
      if (errorMsg.includes('leader') || errorMsg.includes('Leader')) {
        return c.json({ error: errorMsg }, 403);
      }
      // その他のエラーは 409 または 400
      if (errorMsg.includes('cannot accept') || errorMsg.includes('status')) {
        return c.json({ error: errorMsg }, 409);
      }
      return c.json({ error: errorMsg }, 400);
    }
  });

  return app;
}
