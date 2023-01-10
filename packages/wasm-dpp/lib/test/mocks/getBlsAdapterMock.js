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
  };

  return blsAdapter;
};
