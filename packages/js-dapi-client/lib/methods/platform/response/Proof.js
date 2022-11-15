class Proof {
  /**
   * @param {object} properties
   * @param {Buffer} properties.merkleProof
   * @param {Buffer} properties.signatureLLMQHash
   * @param {Buffer} properties.signature
   * @param {number} properties.round
   */
  constructor(properties) {
    this.merkleProof = properties.merkleProof;
    this.signatureLLMQHash = properties.signatureLLMQHash;
    this.signature = properties.signature;
    this.round = properties.round;
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
   *
   * @returns {number}
   */
  getRound() {
    return this.round;
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
      round: proofProto.getRound(),
    });
  }
}

module.exports = Proof;
