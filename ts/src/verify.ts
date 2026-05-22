import nacl from 'tweetnacl';
import { canonicalize } from './sign';

export function verifyOperation(opJson: string, publicKeyB64: string): boolean {
  const canonical = canonicalize(opJson);
  const obj = JSON.parse(opJson);
  const sigB64 = obj.signature ?? '';

  try {
    const message = new TextEncoder().encode(canonical);
    const sig = Buffer.from(sigB64, 'base64');
    const pk = Buffer.from(publicKeyB64, 'base64');
    return nacl.sign.detached.verify(message, sig, pk);
  } catch {
    return false;
  }
}
