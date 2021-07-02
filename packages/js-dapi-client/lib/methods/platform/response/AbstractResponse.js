/**
 * @abstract
 */
class AbstractResponse {
  /**
   * @param {Metadata} metadata
   * @param {Proof} [proof]
   */
  constructor(metadata, proof = undefined) {
    this.metadata = metadata;
    this.proof = proof;
  }

  /**
   * @returns {Metadata} - metadata
   */
  getMetadata() {
    return this.metadata;
  }

  /**
   * @returns {Proof} - data with required information for cryptographical verification
   */
  getProof() {
    return this.proof;
  }
}

module.exports = AbstractResponse;
