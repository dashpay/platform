const AbstractResponse = require('../response/AbstractResponse');
const InvalidResponseError = require('../response/errors/InvalidResponseError');

class GetIdentityResponse extends AbstractResponse {
  /**
   * @param {Buffer} identity
   * @param {Metadata} metadata
   * @param {Proof} [proof]
   */
  constructor(identity, metadata, proof = undefined) {
    super(metadata, proof);

    this.identity = identity;
  }

  /**
   * @returns {Buffer}
   */
  getIdentity() {
    return this.identity;
  }

  /**
   * @param proto
   * @returns {GetIdentityResponse}
   */
  static createFromProto(proto) {
    const identity = proto.getIdentity();
    const { metadata, proof } = AbstractResponse.createMetadataAndProofFromProto(proto);

    if (!identity && !proof) {
      throw new InvalidResponseError('Identity is not defined');
    }

    return new GetIdentityResponse(
      Buffer.from(proto.getIdentity()),
      metadata,
      proof,
    );
  }
}

module.exports = GetIdentityResponse;
