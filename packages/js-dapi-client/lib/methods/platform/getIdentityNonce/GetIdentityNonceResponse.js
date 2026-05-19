import AbstractResponse from '../response/AbstractResponse.js';
import InvalidResponseError from '../response/errors/InvalidResponseError.js';

const IDENTITY_NONCE_VALUE_FILTER = BigInt(0xFFFFFFFFFF);

class GetIdentityNonceResponse extends AbstractResponse {
  /**
   * @param {bigint} identityNonce
   * @param {Metadata} metadata
   * @param {Proof} [proof]
   */
  constructor(identityNonce, metadata, proof = undefined) {
    super(metadata, proof);

    this.identityNonce = identityNonce;
  }

  /**
   * @returns {bigint}
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
    const identityNonce = BigInt(proto.getV0()
      .getIdentityNonce()) & IDENTITY_NONCE_VALUE_FILTER;
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

export default GetIdentityNonceResponse;
