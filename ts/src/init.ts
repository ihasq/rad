import * as fs from 'fs';
import * as path from 'path';

export interface InitResult {
  founder: string;
  publicKey: string;
}

export function initProject(
  dir: string,
  participantId: string,
  publicKey: string
): InitResult {
  const radDir = path.join(dir, '.rad');

  if (fs.existsSync(radDir)) {
    throw new Error('Already initialized');
  }

  fs.mkdirSync(radDir, { recursive: true });

  // config.json
  const config = {
    founder: participantId,
    publicKey: publicKey
  };
  fs.writeFileSync(
    path.join(radDir, 'config.json'),
    JSON.stringify(config)
  );

  // participants.json
  const participants = [{
    id: participantId,
    publicKey: publicKey
  }];
  fs.writeFileSync(
    path.join(radDir, 'participants.json'),
    JSON.stringify(participants)
  );

  // empty oplog + regions
  fs.writeFileSync(path.join(radDir, 'oplog.json'), '[]');
  fs.writeFileSync(path.join(radDir, 'regions.json'), '[]');

  // founders.json with root founder
  const founders = {
    '.': participantId
  };
  fs.writeFileSync(
    path.join(radDir, 'founders.json'),
    JSON.stringify(founders)
  );

  return {
    founder: participantId,
    publicKey: publicKey
  };
}
