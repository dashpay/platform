const AbstractResponse = require('../response/AbstractResponse');
const InvalidResponseError = require('../response/errors/InvalidResponseError');

const IDENTITY_NONCE_VALUE_FILTER = 0xFFFFFFFFFF;

class GetIdentityNonceResponse extends AbstractResponse {
  /**
   * @param {number} identityNonce
   * @param {Metadata} metadata
   * @param {Proof} [proof]
   */
  constructor(identityNonce, metadata, proof = undefined) {
    super(metadata, proof);

    this.identityNonce = identityNonce;
  }

  /**
   * @returns {number}
   */
  getIdentityNonce() {
    return this.identityNonce;
  }

  /**
   * @param proto
   * @returns {GetIdentityNonceResponse}
   */
  static createFromProto(proto) {
    // eslint-disable-next-line
    const identityNonce = proto.getV0()
      .getIdentityNonce() & IDENTITY_NONCE_VALUE_FILTER;
    const { metadata, proof } = AbstractResponse.createMetadataAndProofFromProto(
      proto,
    );

    if ((typeof identityNonce === 'undefined' || identityNonce === null) && !proof) {
      throw new InvalidResponseError('Nonce data is not defined');
    }

    return new GetIdentityNonceResponse(
      identityNonce,
      metadata,
      proof,
    );
  }
}

module.exports = GetIdentityNonceResponse;
