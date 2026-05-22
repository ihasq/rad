import { Hono } from 'hono';
import { OpLog } from '../oplog';
import { RegionMap } from '../region';
import { FounderTree } from '../founder';
import { createParticipantsRoutes } from './routes/participants';
import { createOperationsRoutes } from './routes/operations';
import { createAcceptRoutes } from './routes/accept';
import { createReadRoutes } from './routes/read';
import type { S3RadStore } from '../storage/s3-store';

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
  store?: S3RadStore;  // Optional S3 store for persistence
}

export async function createRelayApp(store?: S3RadStore) {
  const app = new Hono();

  // Initialize state
  const state: RelayState = {
    oplog: new OpLog(),
    regionMap: new RegionMap(),
    participants: new Map(),
    founderTree: new FounderTree(''),
    store,
  };

  // If S3 store is provided, load existing data
  if (store) {
    try {
      // Load operations
      const operations = await store.loadOplog();
      for (const op of operations) {
        state.oplog.append(op);
      }

      // Load regions
      const regions = await store.loadRegions();
      for (const region of regions) {
        state.regionMap.register(region);
      }

      // Load participants
      const participants = await store.loadParticipants();
      for (const p of participants) {
        // Convert types::Participant to relay::Participant
        state.participants.set(p.id, {
          participantId: p.id,
          publicKey: p.publicKey,
          displayName: p.displayName,
          isFounder: false,  // Will be determined by founder tree
          joinedAt: p.joinedAt,
        });
      }

      // Load founders
      const foundersData = await store.loadFounders();
      state.founderTree.loadFoundersObject(foundersData.founders);
    } catch (e) {
      console.error('Failed to load data from S3:', e);
      throw e;
    }
  }

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
