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
    // Use _asU8 so we get bytes regardless of the underlying protobuf
    // representation (grpc-js: Uint8Array; grpc-web: base64 string).
    // new Uint8Array(string) does NOT base64-decode, silently losing bytes.
    return new Proof({
      merkleProof: proofProto.getGrovedbProof_asU8(),
      quorumHash: proofProto.getQuorumHash_asU8(),
      signature: proofProto.getSignature_asU8(),
      round: proofProto.getRound(),
    });
  }
}

module.exports = Proof;
