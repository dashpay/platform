const StoreTreeProofs = require('./StoreTreeProofs');

class Proof {
  /**
   * @param {object} properties
   * @param {Buffer} properties.rootTreeProof
   * @param {StoreTreeProofs} properties.storeTreeProofs
   * @param {Buffer} properties.signatureLLMQHash
   * @param {Buffer} properties.signature
   */
  constructor(properties) {
    this.rootTreeProof = properties.rootTreeProof;
    this.storeTreeProofs = properties.storeTreeProofs;
    this.signatureLLMQHash = properties.signatureLLMQHash;
    this.signature = properties.signature;
  }

  /**
   * @returns {Buffer}
   */
  getRootTreeProof() {
    return this.rootTreeProof;
  }

  /**
   * @returns {StoreTreeProofs}
   */
  getStoreTreeProofs() {
    return this.storeTreeProofs;
  }

  /**
   * @returns {Buffer}
   */
  getSignatureLLMQHash() {
    return this.signatureLLMQHash;
  }

  /**
   * @returns {Buffer}
   */
  getSignature() {
    return this.signature;
  }

  /**
   * @param proofProto
   *
   * @returns {Proof}
   */
  static createFromProto(proofProto) {
    const rawStoreProofs = proofProto.getStoreTreeProofs();

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
      rootTreeProof: Buffer.from(proofProto.getRootTreeProof()),
      storeTreeProofs,
      signatureLLMQHash: Buffer.from(proofProto.getSignatureLlmqHash()),
      signature: Buffer.from(proofProto.getSignature()),
    });
  }
}

module.exports = Proof;
