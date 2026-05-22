import * as fs from 'fs';
import * as path from 'path';
import type { OpLog } from './oplog';
import type { RegionMap } from './region';
import type { FounderTree } from './founder';
import type { Operation, CodeRegion, OpStatus, Participant } from './types';

export class RadStore {
  private radDir: string;

  private constructor(radDir: string) {
    this.radDir = radDir;
  }

  static open(projectDir: string): RadStore {
    const radDir = path.join(projectDir, '.rad');
    if (!fs.existsSync(radDir)) {
      throw new Error('Not a rad project (run rad init first)');
    }
    return new RadStore(radDir);
  }

  loadOplog(): Operation[] {
    const oplogPath = path.join(this.radDir, 'oplog.json');
    if (!fs.existsSync(oplogPath)) {
      return [];
    }

    try {
      const content = fs.readFileSync(oplogPath, 'utf-8');
      return JSON.parse(content);
    } catch (e) {
      throw new Error(`error: corrupt or invalid oplog.json: ${e}`);
    }
  }

  saveOplog(operations: Operation[]): void {
    const oplogPath = path.join(this.radDir, 'oplog.json');
    const json = JSON.stringify(operations);
    fs.writeFileSync(oplogPath, json);
  }

  loadRegions(): CodeRegion[] {
    const regionsPath = path.join(this.radDir, 'regions.json');
    if (!fs.existsSync(regionsPath)) {
      return [];
    }

    try {
      const content = fs.readFileSync(regionsPath, 'utf-8');
      return JSON.parse(content);
    } catch {
      return [];
    }
  }

  saveRegions(regions: CodeRegion[]): void {
    const regionsPath = path.join(this.radDir, 'regions.json');
    const json = JSON.stringify(regions);
    fs.writeFileSync(regionsPath, json);
  }

  loadParticipants(): Participant[] {
    const participantsPath = path.join(this.radDir, 'participants.json');
    if (!fs.existsSync(participantsPath)) {
      return [];
    }

    try {
      const content = fs.readFileSync(participantsPath, 'utf-8');
      return JSON.parse(content);
    } catch {
      return [];
    }
  }

  saveParticipants(participants: Participant[]): void {
    const participantsPath = path.join(this.radDir, 'participants.json');
    const json = JSON.stringify(participants);
    fs.writeFileSync(participantsPath, json);
  }

  loadFounders(): { founders: Record<string, string>; rootFounder: string } {
    const configPath = path.join(this.radDir, 'config.json');
    let rootFounder = '';
    if (fs.existsSync(configPath)) {
      try {
        const content = fs.readFileSync(configPath, 'utf-8');
        const config = JSON.parse(content);
        rootFounder = config.founder || '';
      } catch {}
    }

    const foundersPath = path.join(this.radDir, 'founders.json');
    if (fs.existsSync(foundersPath)) {
      try {
        const content = fs.readFileSync(foundersPath, 'utf-8');
        const founders = JSON.parse(content);
        return { founders, rootFounder };
      } catch {}
    }

    return { founders: { '.': rootFounder }, rootFounder };
  }

  saveFounders(founders: Record<string, string>): void {
    const foundersPath = path.join(this.radDir, 'founders.json');
    const json = JSON.stringify(founders);
    fs.writeFileSync(foundersPath, json);
  }

  putSnapshot(filePath: string, content: string): void {
    const snapPath = path.join(this.radDir, 'snapshots', filePath);
    const dir = path.dirname(snapPath);
    fs.mkdirSync(dir, { recursive: true });
    fs.writeFileSync(snapPath, content);
  }

  getSnapshot(filePath: string): string | null {
    const snapPath = path.join(this.radDir, 'snapshots', filePath);
    try {
      return fs.readFileSync(snapPath, 'utf-8');
    } catch {
      return null;
    }
  }

  compact(): void {
    const operations = this.loadOplog();

    // Group operations by file path and collect accepted writes
    const fileContents = new Map<string, string>();
    const acceptedIds = new Set<string>();

    for (const op of operations) {
      if (op.status === 'accepted' && op.op_type === 'Write') {
        // Extract file path from region_id (format: "file:start-end")
        const colonPos = op.region_id.indexOf(':');
        if (colonPos !== -1) {
          const filePath = op.region_id.substring(0, colonPos);
          fileContents.set(filePath, op.content);
          acceptedIds.add(op.id);
        }
      }
    }

    // Write snapshots
    for (const [filePath, content] of fileContents.entries()) {
      this.putSnapshot(filePath, content);
    }

    // Remove accepted operations from oplog
    const newOperations = operations.filter(op => !acceptedIds.has(op.id));
    this.saveOplog(newOperations);
  }
}
