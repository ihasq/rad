import type { Operation, OpType, OpStatus } from './types';
import type { OpLog } from './oplog';
import type { FounderTree } from './founder';
import { signOperation } from './sign';

export interface DeleteResult {
  operationId: string;
  status: string;
  filePath: string;
}

export function handleDelete(
  filePath: string,
  participant: string,
  secretKey: string,
  founderTree: FounderTree,
  oplog: OpLog
): DeleteResult {
  // region_id はファイル全体を表す特別なID
  const regionId = `file:${filePath}`;

  // Operation 生成
  const opId = `op-${participant}-${oplog.getAllOperations().length}`;
  const timestamp = Date.now();

  const operation: Operation = {
    id: opId,
    participantId: participant,
    regionId,
    type: 'delete' as OpType,
    content: '',
    signature: '',
    timestamp,
    status: 'visible' as OpStatus,
  };

  // JSON 正規化 → 署名
  const opJson = JSON.stringify(operation);
  const signature = signOperation(opJson, secretKey);

  // 署名を追加
  const signedOp: Operation = { ...operation, signature };

  // oplog に追加
  oplog.append(signedOp);

  return {
    operationId: opId,
    status: 'visible',
    filePath,
  };
}

export function canAcceptDelete(
  filePath: string,
  accepter: string,
  founderTree: FounderTree
): boolean {
  const fileFounder = founderTree.getFileFounder(filePath);
  return fileFounder === accepter;
}
