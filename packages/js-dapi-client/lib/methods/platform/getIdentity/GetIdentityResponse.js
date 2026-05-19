const AbstractResponse = require('../response/AbstractResponse');
const InvalidResponseError = require('../response/errors/InvalidResponseError');

class GetIdentityResponse extends AbstractResponse {
  /**
   * @param {Uint8Array} identity
   * @param {Metadata} metadata
   * @param {Proof} [proof]
   */
  constructor(identity, metadata, proof = undefined) {
    super(metadata, proof);

    this.identity = identity;
  }

  /**
   * @returns {Uint8Array}
   */
  getIdentity() {
    return this.identity;
  }

  /**
   * @param proto
   * @returns {GetIdentityResponse}
   */
  static createFromProto(proto) {
    const identity = proto.getV0().getIdentity();
    const { metadata, proof } = AbstractResponse.createMetadataAndProofFromProto(proto);

    if (!identity && !proof) {
      throw new InvalidResponseError('Identity is not defined');
    }

    return new GetIdentityResponse(
      new Uint8Array(proto.getV0().getIdentity()),
      metadata,
      proof,
    );
  }
}

module.exports = GetIdentityResponse;
