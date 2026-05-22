import type { Operation } from './types';

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
}
