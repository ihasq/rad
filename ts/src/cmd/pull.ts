import { RemoteClient } from '../remote';
import { RadStore } from '../store';
import * as fs from 'fs';
import * as path from 'path';

interface RemoteConfig {
  url: string;
  lastPullTimestamp: number;
}

export async function runPull(store: RadStore): Promise<string> {
  // Load remote.json
  const remotePath = path.join('.rad', 'remote.json');
  const remoteJson = fs.readFileSync(remotePath, 'utf-8');
  const remoteConfig: RemoteConfig = JSON.parse(remoteJson);

  const client = new RemoteClient(remoteConfig.url);

  // Get new operations since last pull
  const newOps = await client.getLog(remoteConfig.lastPullTimestamp);

  if (newOps.length === 0) {
    return 'pulled: 0 operations (already up to date)\n';
  }

  // Load existing oplog
  const existingOps = store.loadOplog();
  const existingIds = new Set(existingOps.map(o => o.id));

  // Add new operations (deduplication by ID)
  const added: Array<{ id: string; status: string; participantId: string; regionId: string }> = [];
  const toAdd = [];

  for (const op of newOps) {
    if (!existingIds.has(op.id)) {
      added.push({
        id: op.id,
        status: op.status,
        participantId: op.participantId,
        regionId: op.regionId,
      });
      toAdd.push(op);
    }
  }

  if (added.length === 0) {
    return 'pulled: 0 new operations (duplicates filtered)\n';
  }

  // Save updated oplog
  const allOps = [...existingOps, ...toAdd];
  store.saveOplog(allOps);

  // Update lastPullTimestamp
  const maxTimestamp = Math.max(...allOps.map(o => o.timestamp));
  remoteConfig.lastPullTimestamp = maxTimestamp;

  fs.writeFileSync(remotePath, JSON.stringify(remoteConfig));

  // Build output
  let output = '';
  output += `pulled: ${added.length} operations from ${remoteConfig.url}\n`;

  for (const { id, status, participantId, regionId } of added) {
    // Extract file from regionId
    const colonPos = regionId.indexOf(':');
    const file = colonPos !== -1 ? regionId.substring(0, colonPos) : regionId;

    output += `  ${id} [${status}] ${file} by ${participantId}\n`;
  }

  return output;
}
