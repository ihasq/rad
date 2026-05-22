import { spawn } from 'bun';
import { RadStore } from '../store';
import { RegionMap } from '../region';
import type { Operation, Participant, CodeRegion } from '../types';

export interface ImportResult {
  commitCount: number;
  operationCount: number;
  participantCount: number;
}

export async function importFromGit(projectDir: string): Promise<ImportResult> {
  const store = RadStore.open(projectDir);

  // Load existing state
  const oplog: Operation[] = store.loadOplog();
  const regionMap = new RegionMap();
  regionMap.loadRegions(store.loadRegions());

  // Track participants
  const participants: Participant[] = [];
  const participantMap = new Map<string, string>();

  // Get git log
  const proc = spawn(['git', 'log', '--format=%H|%ae|%an|%at|%s', '--reverse', '--name-status'], {
    cwd: projectDir,
  });

  const output = await new Response(proc.stdout).text();
  const lines = output.split('\n');

  let commitCount = 0;
  let operationCount = 0;
  let i = 0;

  while (i < lines.length) {
    const line = lines[i].trim();

    // Parse commit line
    if (line.includes('|')) {
      const parts = line.split('|');
      if (parts.length >= 5) {
        const commitHash = parts[0];
        const authorEmail = parts[1];
        const authorName = parts[2];
        const timestamp = parseInt(parts[3]) * 1000; // Convert to ms
        const _message = parts[4];

        // Register participant
        const participantId = authorName;
        if (!participantMap.has(participantId)) {
          participantMap.set(participantId, authorEmail);
          participants.push({
            id: participantId,
            publicKey: `git-import-${commitHash.substring(0, 7)}`,
            displayName: authorName,
            joinedAt: timestamp,
          });
        }

        // Get changed files from following lines
        i++;

        // Skip empty line after commit info
        if (i < lines.length && lines[i].trim() === '') {
          i++;
        }

        while (i < lines.length) {
          const fileLine = lines[i].trim();

          // Stop at next commit or empty line
          if (fileLine === '' || fileLine.includes('|')) {
            break;
          }

          // Parse file status line (e.g., "M\tsrc/main.ts")
          const fileParts = fileLine.split(/\s+/);
          if (fileParts.length >= 2) {
            const _status = fileParts[0]; // M, A, D, etc.
            const filePath = fileParts[1];

            // Get file content at this commit
            try {
              const contentProc = spawn(['git', 'show', `${commitHash}:${filePath}`], {
                cwd: projectDir,
              });
              const content = await new Response(contentProc.stdout).text();

              if (content) {
                // Calculate line count (match Rust's lines().count() behavior)
                const lines = content.split('\n');
                const lineCount = Math.max(1, lines[lines.length - 1] === '' ? lines.length - 1 : lines.length);
                const regionId = `${filePath}:1-${lineCount}`;

                // Register region (only if not already registered)
                const region: CodeRegion = {
                  id: regionId,
                  filePath: filePath,
                  startLine: 1,
                  endLine: lineCount,
                  ownerId: participantId,
                };
                regionMap.register(region);

                // Create operation
                const opId = `op-${timestamp}-${operationCount}`;
                const operation: Operation = {
                  id: opId,
                  participantId: participantId,
                  regionId: regionId,
                  type: 'write',
                  content: content,
                  reason: undefined,
                  signature: 'git-imported',
                  timestamp: timestamp,
                  status: 'accepted',
                };

                oplog.push(operation);
                operationCount++;
              }
            } catch (e) {
              // File doesn't exist at this commit, skip
            }
          }

          i++;
        }

        commitCount++;
        continue;
      }
    }

    i++;
  }

  // Save state
  store.saveOplog(oplog);
  store.saveRegions(regionMap.getAllRegions());
  store.saveParticipants(participants);

  return {
    commitCount,
    operationCount,
    participantCount: participants.length,
  };
}
