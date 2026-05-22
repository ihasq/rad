import { RemoteClient } from '../remote';
import { RadStore } from '../store';
import * as fs from 'fs';
import * as path from 'path';

interface RemoteConfig {
  url: string;
}

export async function runPush(store: RadStore): Promise<string> {
  // Load remote.json
  const remotePath = path.join('.rad', 'remote.json');
  const remoteJson = fs.readFileSync(remotePath, 'utf-8');
  const remoteConfig: RemoteConfig = JSON.parse(remoteJson);

  const client = new RemoteClient(remoteConfig.url);

  // Load operations
  const operations = store.loadOplog();

  // Track pushed and skipped operations
  const pushedOps: Array<{ id: string; status: string; regionId: string }> = [];
  let skipped = 0;

  // Submit each operation
  for (const op of operations) {
    try {
      await client.submitOperation(op);
      pushedOps.push({ id: op.id, status: op.status, regionId: op.regionId });
    } catch (error) {
      const errorMsg = error instanceof Error ? error.message : String(error);
      // If it's a duplicate, that's okay (idempotency)
      if (errorMsg.includes('409') || errorMsg.includes('already exists')) {
        skipped++;
      } else {
        // Other errors we should report but not fail
        console.error(`Warning: Failed to push ${op.id}: ${errorMsg}`);
      }
    }
  }

  // Build output
  let output = '';
  output += `pushed: ${pushedOps.length} operations to ${remoteConfig.url}\n`;

  for (const { id, status, regionId } of pushedOps) {
    // Extract file from regionId
    const colonPos = regionId.indexOf(':');
    const file = colonPos !== -1 ? regionId.substring(0, colonPos) : regionId;

    output += `  ${id} [${status}] ${file}\n`;
  }

  if (skipped > 0) {
    output += `  (${skipped} operations already on server)\n`;
  }

  return output;
}
