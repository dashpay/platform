const AbstractResponse = require('../response/AbstractResponse');
const Metadata = require('../response/Metadata');
const Proof = require('../response/Proof');
const ErrorResult = require('./ErrorResult');

class WaitForStateTransitionResultResponse extends AbstractResponse {
  /**
   * @param {Metadata} metadata
   * @param {Proof} [proof]
   * @param {ErrorResult} [error]
   * @param {bigint} [ownerBalance]
   */
  constructor(metadata, proof = undefined, error = undefined, ownerBalance = undefined) {
    super(metadata, proof);

    this.error = error;
    this.ownerBalance = ownerBalance;
  }

  /**
   * @returns {ErrorResult}
   */
  getError() {
    return this.error;
  }

  /**
   * The credit balance of the transition's owner after it executed, as DAPI
   * read it without a proof. Set when the request asked for the user's
   * balance and for no proof; a proved response of an owned, fee-paying
   * transition carries the balance inside the proof.
   *
   * @returns {bigint|undefined}
   */
  getOwnerBalance() {
    return this.ownerBalance;
  }

  /**
   * @param proto
   * @returns {WaitForStateTransitionResultResponse}
   */
  static createFromProto(proto) {
    let error;
    let proof;

    if (proto.getV0().getProof()) {
      proof = Proof.createFromProto(proto.getV0().getProof());
    }

    if (proto.getV0().getError()) {
      let data;

      if (proto.getV0().getError().getData()) {
        data = Buffer.from(proto.getV0().getError().getData());
      }

      error = new ErrorResult(
        proto.getV0().getError().getCode(),
        proto.getV0().getError().getMessage(),
        data,
      );
    }

    const metadata = proto.getV0().getMetadata()
      ? new Metadata(proto.getV0().getMetadata().toObject()) : null;

    let ownerBalance;
    const successWithOwnerBalance = proto.getV0().getSuccessWithOwnerBalance();
    if (successWithOwnerBalance) {
      ownerBalance = BigInt(successWithOwnerBalance.getOwnerBalance());
    }

    return new WaitForStateTransitionResultResponse(
      metadata,
      proof,
      error,
      ownerBalance,
    );
  }
}

module.exports = WaitForStateTransitionResultResponse;
