import type { RadStore } from '../store';

export function runDiff(store: RadStore): string {
  const operations = store.loadOplog();

  // Get visible writes
  const visibleWrites = operations.filter(o => o.status === 'visible' && o.type === 'write');

  if (visibleWrites.length === 0) {
    return 'no pending changes\n';
  }

  // Build a map of accepted state (file -> latest accepted content)
  const acceptedState = new Map<string, string>();
  const opsByTime = [...operations].sort((a, b) => a.timestamp - b.timestamp);

  for (const op of opsByTime) {
    if (op.status === 'accepted' && op.type === 'write') {
      // Extract file path from regionId (format: "file:start-end")
      const colonPos = op.regionId.indexOf(':');
      if (colonPos !== -1) {
        const filePath = op.regionId.substring(0, colonPos);
        acceptedState.set(filePath, op.content);
      }
    }
  }

  // Build output
  let output = '';
  for (const op of visibleWrites) {
    // Extract file path from regionId
    const colonPos = op.regionId.indexOf(':');
    const filePath = colonPos !== -1 ? op.regionId.substring(0, colonPos) : op.regionId;

    // Check if this is a new file
    const isNewFile = !acceptedState.has(filePath);

    if (isNewFile) {
      output += '--- (new file)\n';
      output += `+++ visible by ${op.participantId} (${op.id})\n`;
      output += `+ ${op.regionId}  "${op.content}"\n`;
    } else {
      const acceptedContent = acceptedState.get(filePath)!;
      output += `--- accepted (${filePath})\n`;
      output += `+++ visible by ${op.participantId} (${op.id})\n`;
      output += `-${acceptedContent}\n`;
      output += `+${op.content}\n`;
    }
    output += '\n';
  }

  return output;
}
