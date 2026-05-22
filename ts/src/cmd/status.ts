import type { RadStore } from '../store';

export function runStatus(store: RadStore): string {
  // Load all state
  const operations = store.loadOplog();
  const participants = store.loadParticipants();
  const regions = store.loadRegions();
  const foundersData = store.loadFounders();

  // Get founder
  const founder = foundersData.rootFounder || '(unknown)';

  // Count operations by status
  const total = operations.length;
  const accepted = operations.filter(o => o.status === 'accepted').length;
  const visible = operations.filter(o => o.status === 'visible').length;
  const rejected = operations.filter(o => o.status === 'rejected').length;
  const discarded = operations.filter(o => o.status === 'discarded').length;

  // Count regions and files
  const regionCount = regions.length;
  const files = new Set(regions.map(r => r.filePath));
  const fileCount = files.size;

  // Get visible writes awaiting review
  const visibleWrites = operations.filter(o => o.status === 'visible' && o.type === 'write');

  // Build output
  let output = '';
  output += `rad project: . (founder: ${founder})\n`;
  output += `participants: ${participants.length}\n`;
  output += `operations: ${total} (${accepted} accepted, ${visible} visible, ${rejected} rejected, ${discarded} discarded)\n`;
  output += `regions: ${regionCount}\n`;
  output += `files: ${fileCount}\n`;

  if (visibleWrites.length > 0) {
    output += '\nvisible writes awaiting review:\n';
    for (const op of visibleWrites) {
      // Extract content preview (first 50 chars, escape newlines)
      const contentClean = op.content.replace(/\n/g, '\\n').replace(/\r/g, '');
      const contentPreview = contentClean.length > 50
        ? `"${contentClean.slice(0, 47)}..."`
        : `"${contentClean}"`;
      output += `  ${op.id} [visible] ${op.participantId}  ${op.regionId}  ${contentPreview}\n`;
    }
  }

  return output;
}
