import { Hono } from 'hono';
import { OpLog } from '../oplog';
import { RegionMap } from '../region';
import { FounderTree } from '../founder';
import { createParticipantsRoutes } from './routes/participants';
import { createOperationsRoutes } from './routes/operations';
import { createAcceptRoutes } from './routes/accept';
import { createReadRoutes } from './routes/read';

export interface Participant {
  participantId: string;
  publicKey: string;
  displayName?: string;
  isFounder: boolean;
  joinedAt: number;
}

export interface RelayState {
  oplog: OpLog;
  regionMap: RegionMap;
  participants: Map<string, Participant>;
  founderTree: FounderTree;
}

export function createRelayApp() {
  const app = new Hono();

  // インメモリ状態
  const state: RelayState = {
    oplog: new OpLog(),
    regionMap: new RegionMap(),
    participants: new Map(),
    founderTree: new FounderTree(''),
  };

  // 参加者エンドポイント
  const participantsRoutes = createParticipantsRoutes(state);
  app.route('/', participantsRoutes);

  // 操作エンドポイント
  const operationsRoutes = createOperationsRoutes(state);
  app.route('/', operationsRoutes);

  // accept エンドポイント
  const acceptRoutes = createAcceptRoutes(state);
  app.route('/', acceptRoutes);

  // 読み取りエンドポイント
  const readRoutes = createReadRoutes(state);
  app.route('/', readRoutes);

  return { app, state };
}
