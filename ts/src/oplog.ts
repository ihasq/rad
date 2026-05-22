import type { Operation, OpStatus } from './types';

export class OpLog {
  private ops: Operation[] = [];

  append(op: Operation) {
    this.ops.push(op);
  }

  getChain(file: string, start: number, end: number): Operation[] {
    const regionId = file + ':' + start + '-' + end;
    return this.ops
      .filter(op => op.regionId === regionId)
      .sort((a, b) => a.timestamp - b.timestamp);
  }

  all(): Operation[] {
    return [...this.ops];
  }

  getById(id: string): Operation | undefined {
    return this.ops.find(op => op.id === id);
  }

  setStatus(id: string, status: OpStatus) {
    const op = this.ops.find(op => op.id === id);
    if (op) {
      op.status = status;
    }
  }

  getChainByRegionId(regionId: string): Operation[] {
    return this.ops
      .filter(op => op.regionId === regionId)
      .sort((a, b) => a.timestamp - b.timestamp);
  }

  len(): number {
    return this.ops.length;
  }
}
