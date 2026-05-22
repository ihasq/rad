import { Hono } from 'hono';
import type { RelayState, Participant } from '../app';

export function createParticipantsRoutes(state: RelayState) {
  const app = new Hono();

  // POST /rad/participants - 参加登録
  app.post('/rad/participants', async (c) => {
    const body = await c.req.json();

    // publicKey 検証
    if (!body.publicKey) {
      return c.json({ error: 'publicKey is required' }, 400);
    }

    // participantId: リクエストから取得、なければ displayName、それもなければ生成
    const participantId = body.participantId || body.displayName || `participant-${Date.now()}`;

    // 最初の参加者は Founder（またはリクエストで明示的に指定）
    const isFounder = body.isFounder !== undefined ? body.isFounder : state.participants.size === 0;

    const participant: Participant = {
      participantId,
      publicKey: body.publicKey,
      displayName: body.displayName,
      isFounder,
      joinedAt: Date.now(),
    };

    state.participants.set(participantId, participant);

    // Founder として登録
    if (isFounder) {
      state.founderTree.registerFromWrite('.', participantId);
    }

    return c.json(participant, 201);
  });

  // GET /rad/participants - 参加者一覧
  app.get('/rad/participants', (c) => {
    const participantList = Array.from(state.participants.values());
    return c.json(participantList, 200);
  });

  return app;
}
