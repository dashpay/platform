const AbstractResponse = require('../response/AbstractResponse');
const InvalidResponseError = require('../response/errors/InvalidResponseError');

class GetTotalCreditsInPlatformResponse extends AbstractResponse {
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
  getTotalCreditsInPlatform() {
    return this.totalCreditsOnPlatform;
  }

  /**
   * @param proto
   * @returns {GetTotalCreditsInPlatformResponse}
   */
  static createFromProto(proto) {
    // eslint-disable-next-line
    const totalCreditsOnPlatform = proto.getV0().getCredits();

    const { metadata, proof } = AbstractResponse.createMetadataAndProofFromProto(
      proto,
    );

    if ((typeof totalCreditsOnPlatform === 'undefined' || totalCreditsOnPlatform === null) && !proof) {
      throw new InvalidResponseError('Total Credits on Platform data is not defined');
    }

    return new GetTotalCreditsInPlatformResponse(
      totalCreditsOnPlatform,
      metadata,
      proof,
    );
  }
}

module.exports = GetTotalCreditsInPlatformResponse;
