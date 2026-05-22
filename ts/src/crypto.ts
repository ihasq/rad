import nacl from 'tweetnacl';

export interface KeyPair {
  publicKey: string;
  secretKey: Uint8Array;
}

export function generateKeypair(): KeyPair {
  const kp = nacl.sign.keyPair();
  return {
    publicKey: Buffer.from(kp.publicKey).toString('base64'),
    secretKey: kp.secretKey,
  };
}

export function formatKeypair(kp: KeyPair): string {
  const secretB64 = Buffer.from(kp.secretKey).toString('base64');
  return 'public:  ' + kp.publicKey + '\n' + 'secret:  ' + secretB64;
}
