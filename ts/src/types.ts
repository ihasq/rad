export interface Participant {
  readonly id: string;
  readonly publicKey: string;
  readonly displayName?: string;
  readonly joinedAt: number;
}

export interface CodeRegion {
  readonly id: string;
  readonly filePath: string;
  readonly startLine: number;
  readonly endLine: number;
  readonly ownerId: string;
}

export type OpType = 'write' | 'approve' | 'reject';

export type OpStatus = 'visible' | 'accepted' | 'rejected' | 'discarded';

export interface Operation {
  readonly id: string;
  readonly participantId: string;
  readonly regionId: string;
  readonly type: OpType;
  readonly content: string;
  readonly reason?: string;
  readonly signature: string;
  readonly timestamp: number;
  status: OpStatus;
}
