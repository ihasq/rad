import nacl from 'tweetnacl';

export function canonicalize(opJson: string): string {
  const obj = JSON.parse(opJson);
  delete obj.signature;
  delete obj.status;
  if (obj.reason === undefined) obj.reason = null;
  // キー昇順ソート
  const sorted = Object.keys(obj).sort().reduce((acc, key) => {
    acc[key] = obj[key];
    return acc;
  }, {} as Record<string, unknown>);
  return JSON.stringify(sorted);
}

export function signOperation(opJson: string, secretKeyB64: string): string {
  const canonical = canonicalize(opJson);
  const message = new TextEncoder().encode(canonical);
  const sk = Buffer.from(secretKeyB64, 'base64');
  const sig = nacl.sign.detached(message, sk);
  return Buffer.from(sig).toString('base64');
}

export function injectSignature(opJson: string, signature: string): string {
  const obj = JSON.parse(opJson);
  obj.signature = signature;
  return JSON.stringify(obj);
}
