import secp256k1 from "secp256k1/elliptic";

export function deriveSharedSecret(
  senderPrivateKeyBuffer,
  receiverPublicKeyBuffer
) {
  const point = Buffer.from(receiverPublicKeyBuffer, "base64");

  const scalar = Buffer.from(senderPrivateKeyBuffer, "hex");

  const sharedSecret = secp256k1.ecdh(point, scalar, {}, Buffer.alloc(32));

  return sharedSecret.toString("hex");
}
