import type { RadStore } from '../store';

export interface LogOptions {
  participant?: string;
  file?: string;
  status?: string;
}

export function runLog(store: RadStore, opts: LogOptions): string {
  let operations = store.loadOplog();

  // Sort by timestamp
  operations.sort((a, b) => a.timestamp - b.timestamp);

  // Apply filters
  if (opts.participant) {
    operations = operations.filter(o => o.participantId === opts.participant);
  }

  if (opts.file) {
    operations = operations.filter(o => o.regionId.startsWith(opts.file));
  }

  if (opts.status) {
    const statusLower = opts.status.toLowerCase();
    operations = operations.filter(o => o.status.toLowerCase() === statusLower);
  }

  // Build output
  let output = '';
  for (const op of operations) {
    // Extract content preview (first 50 chars, escape newlines)
    const contentClean = op.content.replace(/\n/g, '\\n').replace(/\r/g, '');
    const contentPreview = contentClean.length > 50
      ? `"${contentClean.slice(0, 47)}..."`
      : `"${contentClean}"`;

    output += `${op.id} [${op.status}]  ${op.participantId}  ${op.regionId}  ${contentPreview}\n`;
  }

  return output;
}
