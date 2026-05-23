// Relay が生成する ID とタイムスタンプ

let opCounter = 0;

export function generateOpId(): string {
  return 'op-' + Date.now() + '-' + (opCounter++);
}

export function generateParticipantId(index: number): string {
  return 'p-' + index;
}

export function currentTimestamp(): number {
  return Date.now();
}
