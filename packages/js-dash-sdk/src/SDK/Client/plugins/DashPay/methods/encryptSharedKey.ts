import secp256k1 from 'secp256k1/elliptic';

export function encryptSharedKey(senderPrivateKeyBuffer, receiverPublicKeyBuffer){
  const point = receiverPublicKeyBuffer;
  const scalar = senderPrivateKeyBuffer;

  const sharedSecret = secp256k1.ecdh(
    point,
    scalar,
  );
  return Buffer.from(sharedSecret).toString('hex');
};
