import loadBLS from '@dashevo/bls';

export default async () => {
  const bls = await loadBLS();

  return {
    validatePublicKey(publicKeyBuffer) {
      let pk;

      try {
        pk = bls.G1Element.fromBytes(Uint8Array.from(publicKeyBuffer));
      } catch (e) {
        return false;
      } finally {
        if (pk) {
          pk.delete();
        }
      }

      return Boolean(pk);
    },
    sign(data, key) {
      const blsKey = bls.PrivateKey.fromBytes(Uint8Array.from(key), true);
      const signature = bls.BasicSchemeMPL.sign(blsKey, data);
      const result = Buffer.from(signature.serialize());

      signature.delete();
      blsKey.delete();

      return result;
    },
    verifySignature(signature, data, publicKey) {
      const { G1Element, G2Element, BasicSchemeMPL } = bls;

      const blsKey = G1Element.fromBytes(Uint8Array.from(publicKey));

      const blsSignature = G2Element.fromBytes(
        Uint8Array.from(signature),
      );

      const result = BasicSchemeMPL.verify(blsKey, Uint8Array.from(data), blsSignature);

      blsKey.delete();
      blsSignature.delete();

      return result;
    },
  };
};
