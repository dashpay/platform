class Proof {
  /**
   * @param {object} properties
   * @param {Uint8Array} properties.merkleProof
   * @param {Uint8Array} properties.quorumHash
   * @param {Uint8Array} properties.signature
   * @param {number} properties.round
   */
  constructor(properties) {
    this.merkleProof = properties.merkleProof;
    this.quorumHash = properties.quorumHash;
    this.signature = properties.signature;
    this.round = properties.round;
  }

  /**
   * @returns {Uint8Array}
   */
  getGrovedbProof() {
    return this.merkleProof;
  }

  /**
   * @returns {Uint8Array}
   */
  getQuorumHash() {
    return this.quorumHash;
  }

  /**
   * @returns {Uint8Array}
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
      merkleProof: new Uint8Array(proofProto.getGrovedbProof()),
      quorumHash: new Uint8Array(proofProto.getQuorumHash()),
      signature: new Uint8Array(proofProto.getSignature()),
      round: proofProto.getRound(),
    });
  }
}

module.exports = Proof;
