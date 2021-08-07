const Proof = require('./Proof');
const StoreTreeProofs = require('./StoreTreeProofs');

/**
 * @param rawProof
 *
 * @returns {Proof}
 */
function createProofFromRawProof(rawProof) {
  const rawStoreProofs = rawProof.getStoreTreeProofs();

  const storeTreeProofs = new StoreTreeProofs({
    dataContractsProof: rawStoreProofs.getDataContractsProof()
      ? Buffer.from(rawStoreProofs.getDataContractsProof()) : null,
    publicKeyHashesToIdentityIdsProof: rawStoreProofs.getPublicKeyHashesToIdentityIdsProof()
      ? Buffer.from(rawStoreProofs.getPublicKeyHashesToIdentityIdsProof()) : null,
    identitiesProof: rawStoreProofs.getIdentitiesProof()
      ? Buffer.from(rawStoreProofs.getIdentitiesProof()) : null,
    documentsProof: rawStoreProofs.getDocumentsProof()
      ? Buffer.from(rawStoreProofs.getDocumentsProof()) : null,
  });

  return new Proof({
    rootTreeProof: Buffer.from(rawProof.getRootTreeProof()),
    storeTreeProofs,
    signatureLLMQHash: Buffer.from(rawProof.getSignatureLlmqHash()),
    signature: Buffer.from(rawProof.getSignature()),
  });
}

module.exports = createProofFromRawProof;
