const cbor = require('cbor');

const AbstractResponse = require('../response/AbstractResponse');
const Metadata = require('../response/Metadata');
const Proof = require('../response/Proof');
const ErrorResult = require('./ErrorResult');

class WaitForStateTransitionResultResponse extends AbstractResponse {
  /**
   * @param {Metadata} metadata
   * @param {Proof} [proof]
   * @param {ErrorResult} [error]
   */
  constructor(metadata, proof = undefined, error = undefined) {
    super(metadata, proof);

    this.error = error;
  }

  /**
   * @returns {ErrorResult}
   */
  getError() {
    return this.error;
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
      const rawData = proto.getV0().getError().getData();
      if (rawData) {
        data = cbor.decode(Buffer.from(rawData));
      }

      error = new ErrorResult(
        proto.getV0().getError().getCode(),
        proto.getV0().getError().getMessage(),
        data,
      );
    }

    const metadata = proto.getV0().getMetadata()
      ? new Metadata(proto.getV0().getMetadata().toObject()) : null;

    return new WaitForStateTransitionResultResponse(
      metadata,
      proof,
      error,
    );
  }
}

module.exports = WaitForStateTransitionResultResponse;
