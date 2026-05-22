import type { CodeRegion, Operation } from './types';
import { RegionMap } from './region';
import { OpLog } from './oplog';
import { signOperation } from './sign';

function generateOpId(): string {
  return 'op-' + Date.now();
}

function currentTimestampMs(): number {
  return Date.now();
}

export function handleWrite(
  parts: string[],
  regionMap: RegionMap,
  oplog: OpLog
): string {
  // parts: write <file> <start> <end> <participant> <secret-key> "<content>"
  const file = parts[1];
  const start = parseInt(parts[2]);
  const end = parseInt(parts[3]);
  const participant = parts[4];
  const secretKey = parts[5];
  const content = parts.slice(6).join(' ').replace(/^"|"$/g, '');
  const regionId = file + ':' + start + '-' + end;

  // 未登録領域なら自動登録（書き込み者が Leader）
  const region: CodeRegion = {
    id: regionId,
    filePath: file,
    startLine: start,
    endLine: end,
    ownerId: participant,
  };
  regionMap.register(region); // 既存なら無視

  // Operation 生成 + 署名
  const opId = generateOpId();
  const timestamp = currentTimestampMs();
  const op: Operation = {
    id: opId,
    participantId: participant,
    regionId: regionId,
    type: 'write',
    content: content,
    reason: undefined,
    signature: '',
    timestamp: timestamp,
  };

  // JSON 正規化 → 署名
  const opJson = JSON.stringify(op);
  const sig = signOperation(opJson, secretKey);
  op.signature = sig;

  oplog.append(op);

  // 出力: JSON with status + chain
  const chain = oplog.getChain(file, start, end).map(o => o.id);
  return JSON.stringify({ id: op.id, status: 'visible', chain: chain });
}

export function handleChain(parts: string[], oplog: OpLog): string {
  const file = parts[1];
  const start = parseInt(parts[2]);
  const end = parseInt(parts[3]);
  const chain = oplog.getChain(file, start, end);

  // ヘッダ
  let result = file + ':' + start + '-' + end + ' (' + chain.length + ' writes, all visible)\n';

  // 各 write の1行表示
  for (const op of chain) {
    result += '  ' + op.id + ' [visible] ' + op.participantId + '  t=' + op.timestamp + '  "' + op.content + '"\n';
  }

  return result.trimEnd();
}
