const AbstractResponse = require('../response/AbstractResponse');
const InvalidResponseError = require('../response/errors/InvalidResponseError');

class GetIdentityBalanceResponse extends AbstractResponse {
  /**
   * @param {Buffer} identity
   * @param {Metadata} metadata
   * @param {Proof} [proof]
   */
  constructor(balance, metadata, proof = undefined) {
    super(metadata, proof);

    this.balance = balance;
  }

  /**
   * @returns {Buffer}
   */
  getIdentity() {
    return this.balance;
  }

  /**
   * @param proto
   * @returns {GetIdentityBalanceResponse}
   */
  static createFromProto(proto) {
    const balance = proto.getV0().getBalance();
    const { metadata, proof } = AbstractResponse.createMetadataAndProofFromProto(proto);

    if (!balance && !proof) {
      throw new InvalidResponseError('Balance is not defined');
    }

    return new GetIdentityBalanceResponse(
      balance,
      metadata,
      proof,
    );
  }
}

module.exports = GetIdentityBalanceResponse;
