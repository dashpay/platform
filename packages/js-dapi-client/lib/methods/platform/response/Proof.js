class Proof {
  /**
   * @param {Object} properties
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
}

module.exports = Proof;
