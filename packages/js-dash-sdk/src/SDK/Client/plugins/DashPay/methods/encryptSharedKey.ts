import secp256k1 from 'secp256k1-native';

export function encryptSharedKey(senderPrivateKeyBuffer, receiverPublicKeyBuffer){
  const ctx = secp256k1.secp256k1_context_create(secp256k1.secp256k1_context_SIGN);

  const sharedSecrect = Buffer.alloc(32);

  const point = receiverPublicKeyBuffer;

  const scalar = senderPrivateKeyBuffer;

  const pubkey64 = Buffer.alloc(64);

  secp256k1.secp256k1_ec_pubkey_parse(ctx, pubkey64, point);

  secp256k1.secp256k1_ecdh(ctx, sharedSecrect, pubkey64, scalar, null);

  return sharedSecrect.toString('hex');
};
