// WASM 結果 → HTTP レスポンス変換層

export interface WasmResult {
  ok: boolean;
  data?: any;
  error?: string;
  code?: string;
}

export function wasmToHttpStatus(result: WasmResult): number {
  if (result.ok) return 200;
  switch (result.code) {
    case 'INVALID_JSON':      return 400;
    case 'MISSING_FIELD':     return 400;
    case 'INVALID_SIGNATURE': return 403;
    case 'NOT_LEADER':        return 403;
    case 'NOT_FOUNDER':       return 403;
    case 'ALREADY_DECIDED':   return 409;
    case 'NOT_FOUND':         return 404;
    default:                  return 500;
  }
}

export function toJoinResponse(data: any) {
  return {
    participantId: data.id,
    publicKey: data.publicKey,
    isFounder: data.isFounder ?? false,
    isMessenger: data.isMessenger ?? false,
    joinedAt: data.joinedAt,
  };
}

export function toSubmitOpResponse(data: any) {
  return {
    operationId: data.id,
    status: data.status,
  };
}

export function toAcceptResponse(data: any) {
  return {
    operationId: data.id,
    status: 'accepted',
    acceptedBy: data.acceptedBy ?? data.by,
    acceptedAt: data.acceptedAt ?? data.at,
  };
}

export function toOpStatusResponse(data: any) {
  return {
    operationId: data.id,
    status: data.status,
    reason: data.reason ?? undefined,
    decidedBy: data.decidedBy ?? undefined,
    decidedAt: data.decidedAt ?? undefined,
  };
}

export function toErrorResponse(result: WasmResult) {
  return {
    error: result.error,
    detail: result.code,
  };
}
