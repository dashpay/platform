const BlsSignatures = require('@dashevo/dpp/lib/bls/bls');

module.exports = async function getBlsAdapterMock() {
  const bls = await BlsSignatures.getInstance();

  const blsAdapter = {
    validatePublicKey(publicKeyBuffer) {
      let pk;

      try {
        pk = bls.PublicKey.fromBytes(publicKeyBuffer);
      } catch (e) {
        return false;
      }

      return Boolean(pk);
    },
    sign(data, key) {
      const blsKey = bls.PrivateKey.fromBytes(key, true);
      const signature = blsKey.sign(data);
      return Buffer.from(signature.serialize());
    },
    verifySignature(signature, data, publicKey) {
      const { PublicKey, Signature: BlsSignature, AggregationInfo } = bls;

      const blsKey = PublicKey.fromBytes(publicKey);

      const aggregationInfo = AggregationInfo.fromMsg(blsKey, data);
      const blsSignature = BlsSignature.fromBytesAndAggregationInfo(signature, aggregationInfo);

      return blsSignature.verify();
    },
  };

  return blsAdapter;
};
