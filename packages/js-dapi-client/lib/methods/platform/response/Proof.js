class Proof {
  /**
   * @param {Object} properties
   * @param {Buffer} properties.rootTreeProof
   * @param {Buffer} properties.storeTreeProof
   * @param {Buffer} properties.signatureLLMQHash
   * @param {Buffer} properties.signature
   */
  constructor(properties) {
    this.rootTreeProof = properties.rootTreeProof;
    this.storeTreeProof = properties.storeTreeProof;
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
   * @returns {Buffer}
   */
  getStoreTreeProof() {
    return this.storeTreeProof;
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
