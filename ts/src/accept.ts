import type { OpStatus } from './types';
import type { OpLog } from './oplog';
import type { RegionMap } from './region';

export interface AcceptResult {
  operationId: string;
  status: string;
  acceptedBy: string;
}

export function handleAccept(
  opId: string,
  leaderId: string,
  regionMap: RegionMap,
  oplog: OpLog
): AcceptResult {
  const op = oplog.getById(opId);
  if (!op) {
    throw new Error('Operation not found');
  }

  // ステータス検証
  if (op.status !== 'visible') {
    throw new Error(`Cannot accept: status is ${op.status}`);
  }

  // Leader 検証
  const owner = regionMap.getOwnerByRegionId(op.regionId);
  if (!owner) {
    throw new Error('Region not found');
  }
  if (owner !== leaderId) {
    throw new Error('Only the Leader can accept');
  }

  // accept 実行
  oplog.setStatus(opId, 'accepted');

  // 階段飛ばし: チェーン内で op より前の visible を discard
  const chain = oplog.getChainByRegionId(op.regionId);
  for (const c of chain) {
    if (c.id === opId) break;
    // Leader 自身の write は discarded にしない
    if (c.status === 'visible' && c.participantId !== leaderId) {
      oplog.setStatus(c.id, 'discarded');
    }
  }

  return {
    operationId: opId,
    status: 'accepted',
    acceptedBy: leaderId,
  };
}
