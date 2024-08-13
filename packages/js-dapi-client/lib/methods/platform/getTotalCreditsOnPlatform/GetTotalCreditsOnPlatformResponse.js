const AbstractResponse = require('../response/AbstractResponse');
const InvalidResponseError = require('../response/errors/InvalidResponseError');

class GetTotalCreditsOnPlatformResponse extends AbstractResponse {
  /**
   * @param {number} totalCreditsOnPlatform
   * @param {Metadata} metadata
   * @param {Proof} [proof]
   */
  constructor(totalCreditsOnPlatform, metadata, proof = undefined) {
    super(metadata, proof);

    this.totalCreditsOnPlatform = totalCreditsOnPlatform;
  }

  /**
   * @returns {number}
   */
  getTotalCreditsOnPlatform() {
    return this.totalCreditsOnPlatform;
  }

  /**
   * @param proto
   * @returns {GetTotalCreditsOnPlatformResponse}
   */
  static createFromProto(proto) {
    // eslint-disable-next-line
    const totalCreditsOnPlatform = proto.getV0().getTotalCreditsOnPlatform();
    const { metadata, proof } = AbstractResponse.createMetadataAndProofFromProto(
      proto,
    );

    if ((typeof totalCreditsOnPlatform === 'undefined' || totalCreditsOnPlatform === null) && !proof) {
      throw new InvalidResponseError('Total Credits on Platform data is not defined');
    }

    return new GetTotalCreditsOnPlatformResponse(
      totalCreditsOnPlatform,
      metadata,
      proof,
    );
  }
}

module.exports = GetTotalCreditsOnPlatformResponse;
