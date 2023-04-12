const loadBLS = require('@dashevo/bls');

module.exports = async function getBlsAdapter() {
  const bls = await loadBLS();
  // const bls = await BlsSignatures.getInstance();

  const blsAdapter = {
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
    privateKeyToPublicKey(privateKeyBuffer) {
      console.log('calling private key to public key');

      const blsKey = bls.PrivateKey.fromBytes(Uint8Array.from(privateKeyBuffer), true);

      const publicKey = blsKey.getG1();

      return publicKey.serialize();
    },
  };

  return blsAdapter;
};
