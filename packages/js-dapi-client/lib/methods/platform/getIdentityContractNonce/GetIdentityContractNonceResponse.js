const AbstractResponse = require('../response/AbstractResponse');
const InvalidResponseError = require('../response/errors/InvalidResponseError');

class GetIdentityContractNonceResponse extends AbstractResponse {
  /**
   * @param {number} identityContractNonce
   * @param {Metadata} metadata
   * @param {Proof} [proof]
   */
  constructor(identityContractNonce, metadata, proof = undefined) {
    super(metadata, proof);

    this.identityContractNonce = identityContractNonce;
  }

  /**
   * @returns {number}
   */
  getIdentityContractNonce() {
    return this.identityContractNonce;
  }

  /**
   * @param proto
   * @returns {GetIdentityContractNonceResponse}
   */
  static createFromProto(proto) {
    const identityContractNonce = proto.getV0().getIdentityContractNonce();
    const { metadata, proof } = AbstractResponse.createMetadataAndProofFromProto(
      proto,
    );

    if ((typeof identityContractNonce === 'undefined' || identityContractNonce === null) && !proof) {
      throw new InvalidResponseError('Contract nonce data is not defined');
    }

    return new GetIdentityContractNonceResponse(
      identityContractNonce,
      metadata,
      proof,
    );
  }
}

module.exports = GetIdentityContractNonceResponse;
