import type { OpLog } from './oplog';
import type { RegionMap } from './region';
import type { FounderTree } from './founder';

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
  founderTree: FounderTree,
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

  const region = regionMap.getById(op.regionId);
  if (!region) {
    throw new Error('Region not found');
  }

  // Founder 階層チェック: 下位 Founder が上位 Founder を reject する場合は reason 必須
  // Get the directory where the operation's file is located
  const opDir = (() => {
    const lastSlash = region.filePath.lastIndexOf('/');
    return lastSlash === -1 ? '.' : region.filePath.substring(0, lastSlash);
  })();

  // Find which directory the rejecter is founder of
  const rejecterFounderDir = (() => {
    const allFounders = founderTree.listAll();
    const entry = allFounders.find(([_, f]) => f === rejecterId);
    return entry ? entry[0] : null;
  })();

  if (rejecterFounderDir) {
    // If rejecter is lower in hierarchy (rej_dir is descendant of op_dir), reason is required
    if (founderTree.isAncestorFounder(opDir, rejecterFounderDir)) {
      // Rejecter is lower-level founder, reason required
      if (!reason || reason.trim() === '') {
        throw new Error('Lower Founder must provide reason to reject upper Founder');
      }
    }
  }

  // Leader → Follower (region-based): reason 必須
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
