const AbstractResponse = require('../response/AbstractResponse');
const InvalidResponseError = require('../response/errors/InvalidResponseError');

class GetTotalCreditsInPlatformResponse extends AbstractResponse {
  /**
   * @param {number} totalCreditsInPlatform
   * @param {Metadata} metadata
   * @param {Proof} [proof]
   */
  constructor(totalCreditsInPlatform, metadata, proof = undefined) {
    super(metadata, proof);

    this.totalCreditsInPlatform = totalCreditsInPlatform;
  }

  /**
   * @returns {number}
   */
  getTotalCreditsInPlatform() {
    return this.totalCreditsInPlatform;
  }

  /**
   * @param proto
   * @returns {GetTotalCreditsInPlatformResponse}
   */
  static createFromProto(proto) {
    // eslint-disable-next-line
    const totalCreditsInPlatform = proto.getV0().getCredits();
    const { metadata, proof } = AbstractResponse.createMetadataAndProofFromProto(
      proto,
    );

    if ((typeof totalCreditsInPlatform === 'undefined' || totalCreditsInPlatform === null) && !proof) {
      throw new InvalidResponseError('Total Credits on Platform data is not defined');
    }

    return new GetTotalCreditsInPlatformResponse(
      totalCreditsInPlatform,
      metadata,
      proof,
    );
  }
}

module.exports = GetTotalCreditsInPlatformResponse;
