import type { OpLog } from './oplog';
import type { RegionMap } from './region';

export interface RejectResult {
  operationId: string;
  status: string;
  rejectedBy: string;
  reason?: string;
}

export function handleReject(
  opId: string,
  rejecterId: string,
  reason: string | undefined,
  regionMap: RegionMap,
  oplog: OpLog
): RejectResult {
  const op = oplog.getById(opId);
  if (!op) {
    throw new Error('Operation not found');
  }

  // ステータス検証
  if (op.status !== 'visible') {
    throw new Error('Cannot reject: not visible');
  }

  // Leader → Follower: reason 必須
  const owner = regionMap.getOwnerByRegionId(op.regionId) || '';
  if (owner === rejecterId && op.participantId !== rejecterId) {
    // rejecter は Leader、対象は Follower
    if (!reason || reason.trim() === '') {
      throw new Error('Leader must provide reason to reject Follower');
    }
  }

  // reject 実行
  oplog.setStatus(opId, 'rejected');

  const result: RejectResult = {
    operationId: opId,
    status: 'rejected',
    rejectedBy: rejecterId,
  };

  if (reason) {
    result.reason = reason;
  }

  return result;
}
