import { spawn } from 'bun';
import { RadStore } from '../store';
import * as path from 'path';
import * as fs from 'fs';

export interface ExportResult {
  commitCount: number;
  operationCount: number;
}

export async function exportToGit(projectDir: string): Promise<ExportResult> {
  const store = RadStore.open(projectDir);

  // Load oplog
  const oplog = store.loadOplog();

  // Get all accepted operations
  const acceptedOps = oplog.filter(op => op.status === 'accepted');

  if (acceptedOps.length === 0) {
    return {
      commitCount: 0,
      operationCount: 0,
    };
  }

  // Group operations by timestamp (within 1 second window)
  const groups: typeof acceptedOps[] = [];
  let currentGroup: typeof acceptedOps = [];
  let lastTimestamp = 0;

  for (const op of acceptedOps) {
    if (currentGroup.length === 0 || (op.timestamp - lastTimestamp) <= 1000) {
      currentGroup.push(op);
      lastTimestamp = op.timestamp;
    } else {
      groups.push(currentGroup);
      currentGroup = [op];
      lastTimestamp = op.timestamp;
    }
  }
  if (currentGroup.length > 0) {
    groups.push(currentGroup);
  }

  let commitCount = 0;
  let operationCount = 0;

  // Create commits for each group
  for (const group of groups) {
    // Write files to working tree
    for (const op of group) {
      // Extract file path from region_id (format: "file:start-end")
      const regionParts = op.regionId.split(':');
      if (regionParts.length >= 1) {
        const filePath = regionParts[0];
        const fullPath = path.join(projectDir, filePath);

        // Create parent directories
        const parentDir = path.dirname(fullPath);
        if (!fs.existsSync(parentDir)) {
          fs.mkdirSync(parentDir, { recursive: true });
        }

        // Write content
        fs.writeFileSync(fullPath, op.content);
        operationCount++;
      }
    }

    // Stage changes
    const addProc = spawn(['git', 'add', '-A'], { cwd: projectDir });
    await addProc.exited;

    if (addProc.exitCode !== 0) {
      throw new Error('git add failed');
    }

    // Check if there are changes to commit
    const statusProc = spawn(['git', 'diff', '--cached', '--quiet'], { cwd: projectDir });
    await statusProc.exited;

    // If exit code is 0, there are no changes
    if (statusProc.exitCode === 0) {
      continue;
    }

    // Collect participants
    const participants: string[] = [];
    const participantSet = new Set<string>();
    for (const op of group) {
      if (!participantSet.has(op.participantId)) {
        participantSet.add(op.participantId);
        participants.push(op.participantId);
      }
    }

    // Create commit message
    const participantList = participants.join(', ');
    let message = `rad: ${group.length} accepted writes by ${participantList}`;

    // Add Co-authored-by if multiple participants
    if (participants.length > 1) {
      for (const participant of participants.slice(1)) {
        message += `\n\nCo-authored-by: ${participant} <${participant}@rad.local>`;
      }
    }

    // Create commit
    const commitProc = spawn(['git', 'commit', '-m', message], { cwd: projectDir });
    await commitProc.exited;

    if (commitProc.exitCode !== 0) {
      const stderr = await new Response(commitProc.stderr).text();
      throw new Error(`git commit failed: ${stderr}`);
    }

    commitCount++;
  }

  return {
    commitCount,
    operationCount,
  };
}
