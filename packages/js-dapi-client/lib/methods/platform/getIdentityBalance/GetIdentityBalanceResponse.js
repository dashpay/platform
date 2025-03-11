const AbstractResponse = require('../response/AbstractResponse');
const InvalidResponseError = require('../response/errors/InvalidResponseError');

class GetIdentityBalanceResponse extends AbstractResponse {
  /**
   * @param {bigint} balance
   * @param {Metadata} metadata
   * @param {Proof} [proof]
   */
  constructor(balance, metadata, proof = undefined) {
    super(metadata, proof);

    this.balance = balance;
  }

  /**
   * @returns {bigint}
   */
  getBalance() {
    return this.balance;
  }

  /**
   * @param proto
   * @returns {GetIdentityBalanceResponse}
   */
  static createFromProto(proto) {
    const balance = BigInt(proto.getV0().getBalance());
    const { metadata, proof } = AbstractResponse.createMetadataAndProofFromProto(proto);

    if ((balance === null || balance === undefined) && !proof) {
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
