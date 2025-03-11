const AbstractResponse = require('../response/AbstractResponse');
const InvalidResponseError = require('../response/errors/InvalidResponseError');

const IDENTITY_CONTRACT_NONCE_VALUE_FILTER = BigInt(0xFFFFFFFFFF);

class GetIdentityContractNonceResponse extends AbstractResponse {
  /**
   * @param {bigint} identityContractNonce
   * @param {Metadata} metadata
   * @param {Proof} [proof]
   */
  constructor(identityContractNonce, metadata, proof = undefined) {
    super(metadata, proof);

    this.identityContractNonce = identityContractNonce;
  }

  /**
   * @returns {bigint}
   */
  getIdentityContractNonce() {
    return this.identityContractNonce;
  }

  /**
   * @param proto
   * @returns {GetIdentityContractNonceResponse}
   */
  static createFromProto(proto) {
    // eslint-disable-next-line
    const identityContractNonce = BigInt(proto.getV0()
      .getIdentityContractNonce()) & IDENTITY_CONTRACT_NONCE_VALUE_FILTER;
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
