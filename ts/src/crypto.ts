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

export function keypairFromSecret(secretB64: string): KeyPair {
  const secretBytes = Buffer.from(secretB64, 'base64');
  // 64バイトフォーマット: 先頭32バイトが秘密鍵、後半32バイトが公開鍵
  const publicKey = Buffer.from(secretBytes.slice(32, 64)).toString('base64');
  return {
    publicKey,
    secretKey: new Uint8Array(secretBytes),
  };
}

export function formatPublicKey(kp: KeyPair): string {
  return kp.publicKey;
}
