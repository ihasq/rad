import { RemoteClient } from '../remote';
import { RadStore } from '../store';
import { keypairFromSecret, formatPublicKey } from '../crypto';
import * as fs from 'fs';
import * as path from 'path';

export async function runClone(url: string, participant: string, secretKey: string): Promise<string> {
  const client = new RemoteClient(url);

  // Generate public key from secret key
  const kp = keypairFromSecret(secretKey);
  const publicKey = formatPublicKey(kp);

  // 1. Join the relay
  await client.join(participant, publicKey, false);

  // 2. Get all participants
  const participants = await client.getParticipants();

  // 3. Get all operations
  const operations = await client.getLog();

  // 4. Create .rad directory
  const radDir = path.join('.', '.rad');
  fs.mkdirSync(radDir, { recursive: true });

  // 5. Determine founder
  const founder = participants.find(p => p.id === 'alice')?.id
    || participants[0]?.id
    || participant;

  // Create config.json
  const config = { founder };
  fs.writeFileSync(
    path.join(radDir, 'config.json'),
    JSON.stringify(config)
  );

  // Open store and save state
  const store = RadStore.open('.');

  // Save participants
  store.saveParticipants(participants);

  // Save operations
  store.saveOplog(operations);

  // Save remote.json
  const lastTimestamp = operations.length > 0
    ? Math.max(...operations.map(o => o.timestamp))
    : 0;

  const remoteConfig = {
    url,
    lastPullTimestamp: lastTimestamp,
  };

  fs.writeFileSync(
    path.join(radDir, 'remote.json'),
    JSON.stringify(remoteConfig)
  );

  // Build response
  let output = '';
  output += `cloned: rad project from ${url}\n`;
  output += `participants: ${participants.length}\n`;
  output += `operations: ${operations.length}\n`;

  // Count unique files
  const files = new Set(
    operations
      .map(o => {
        const colonPos = o.regionId.indexOf(':');
        return colonPos !== -1 ? o.regionId.substring(0, colonPos) : null;
      })
      .filter(f => f !== null)
  );
  output += `files: ${files.size}\n`;

  return output;
}
