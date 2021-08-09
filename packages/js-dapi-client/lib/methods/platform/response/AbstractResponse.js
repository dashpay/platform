const InvalidResponseError = require('./errors/InvalidResponseError');
const Metadata = require('./Metadata');
const Proof = require('./Proof');

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

  /**
   *
   * @param proto
   *
   * @returns{{metadata: Metadata, proof: Proof|undefined}}
   * @throws {InvalidResponseError}
   */
  static createMetadataAndProofFromProto(proto) {
    const metadata = proto.getMetadata();

    if (metadata === undefined) {
      throw new InvalidResponseError('Metadata is not defined');
    }

    const rawProof = proto.getProof();

    let proof;
    if (rawProof) {
      proof = Proof.createFromProto(rawProof);
    }

    return {
      metadata: new Metadata(metadata.toObject()),
      proof,
    };
  }
}

module.exports = AbstractResponse;
