import type { Participant, Operation } from './types';

export interface JoinResponse {
  participantId: string;
}

export interface SubmitResponse {
  id: string;
  status: string;
}

export interface AcceptResponse {
  id: string;
}

export interface FileInfo {
  path: string;
}

export interface FileContent {
  path: string;
  content: string;
}

export class RemoteClient {
  constructor(public url: string) {}

  async join(participantId: string, publicKey: string, isFounder: boolean): Promise<JoinResponse> {
    const response = await fetch(`${this.url}/rad/participants`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ participantId, publicKey, isFounder }),
    });

    if (!response.ok) {
      throw new Error(`Failed to join: ${response.statusText}`);
    }

    return await response.json();
  }

  async getParticipants(): Promise<Participant[]> {
    const response = await fetch(`${this.url}/rad/participants`);

    if (!response.ok) {
      throw new Error(`Failed to get participants: ${response.statusText}`);
    }

    const data: any[] = await response.json();
    return data.map(p => ({
      id: p.participantId,
      publicKey: p.publicKey,
      displayName: p.displayName,
      joinedAt: p.joinedAt,
    }));
  }

  async submitOperation(operation: Operation): Promise<SubmitResponse> {
    const body = {
      id: operation.id,
      participantId: operation.participantId,
      regionId: operation.regionId,
      type: operation.type,
      content: operation.content,
      reason: operation.reason,
      signature: operation.signature,
      timestamp: operation.timestamp,
      status: operation.status,
    };

    const response = await fetch(`${this.url}/rad/operations`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });

    if (!response.ok) {
      const text = await response.text();
      throw new Error(`Failed to submit operation: ${response.statusText} - ${text}`);
    }

    return await response.json();
  }

  async accept(acceptJson: string): Promise<AcceptResponse> {
    const response = await fetch(`${this.url}/rad/accept`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: acceptJson,
    });

    if (!response.ok) {
      throw new Error(`Failed to accept: ${response.statusText}`);
    }

    return await response.json();
  }

  async getLog(since?: number): Promise<Operation[]> {
    let url = `${this.url}/rad/log`;
    if (since !== undefined) {
      url += `?since=${since}`;
    }

    const response = await fetch(url);

    if (!response.ok) {
      throw new Error(`Failed to get log: ${response.statusText}`);
    }

    const data: any[] = await response.json();
    return data.map(o => ({
      id: o.id,
      participantId: o.participantId,
      regionId: o.regionId,
      type: o.type as 'write' | 'approve' | 'reject',
      content: o.content,
      reason: o.reason,
      signature: o.signature,
      timestamp: o.timestamp,
      status: o.status as 'visible' | 'accepted' | 'rejected' | 'discarded',
    }));
  }

  async getFiles(): Promise<string[]> {
    const response = await fetch(`${this.url}/rad/files`);

    if (!response.ok) {
      throw new Error(`Failed to get files: ${response.statusText}`);
    }

    const data: FileInfo[] = await response.json();
    return data.map(f => f.path);
  }

  async getFile(path: string): Promise<FileContent> {
    const response = await fetch(`${this.url}/rad/files/${path}`);

    if (!response.ok) {
      throw new Error(`Failed to get file: ${response.statusText}`);
    }

    return await response.json();
  }
}
