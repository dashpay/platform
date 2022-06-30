import secp256k1 from 'secp256k1/elliptic';

/**
 * Allow to construct a shared secret with the provided sender's private key and the receiver's public key
 * @param senderPrivateKeyBuffer
 * @param receiverPublicKeyBuffer
 */
export function encryptSharedKey(senderPrivateKeyBuffer, receiverPublicKeyBuffer){
  const point = receiverPublicKeyBuffer;
  const scalar = senderPrivateKeyBuffer;

  const sharedSecret = secp256k1.ecdh(
    point,
    scalar,
  );
  return Buffer.from(sharedSecret).toString('hex');
};
