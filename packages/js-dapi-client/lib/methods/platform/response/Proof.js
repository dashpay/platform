class Proof {
  /**
   * @param {object} properties
   * @param {Buffer} properties.merkleProof
   * @param {Buffer} properties.signatureLLMQHash
   * @param {Buffer} properties.signature
   */
  constructor(properties) {
    this.merkleProof = properties.merkleProof;
    this.signatureLLMQHash = properties.signatureLLMQHash;
    this.signature = properties.signature;
  }

  /**
   * @returns {Buffer}
   */
  getMerkleProof() {
    return this.merkleProof;
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
   * @param {Object} proofProto
   *
   * @returns {Proof}
   */
  static createFromProto(proofProto) {
    return new Proof({
      merkleProof: Buffer.from(proofProto.getMerkleProof()),
      signatureLLMQHash: Buffer.from(proofProto.getSignatureLlmqHash()),
      signature: Buffer.from(proofProto.getSignature()),
    });
  }
}

module.exports = Proof;
