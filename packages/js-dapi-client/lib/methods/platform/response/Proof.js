class Proof {
  /**
   * @param {object} properties
   * @param {Buffer} properties.merkleProof
   * @param {Buffer} properties.quorumHash
   * @param {Buffer} properties.signature
   * @param {number} properties.round
   */
  constructor(properties) {
    this.merkleProof = properties.merkleProof;
    this.quorumHash = properties.quorumHash;
    this.signature = properties.signature;
    this.round = properties.round;
  }

  /**
   * @returns {Buffer}
   */
  getGrovedbProof() {
    return this.merkleProof;
  }

  /**
   * @returns {Buffer}
   */
  getQuorumHash() {
    return this.quorumHash;
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
   * @param {object} proofProto
   * @returns {Proof}
   */
  static createFromProto(proofProto) {
    return new Proof({
      merkleProof: Buffer.from(proofProto.getGrovedbProof()),
      quorumHash: Buffer.from(proofProto.getQuorumHash()),
      signature: Buffer.from(proofProto.getSignature()),
      round: proofProto.getRound(),
    });
  }
}

module.exports = Proof;
