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

      if (proto.getV0().getError().getData()) {
        // Use _asU8 so we get bytes regardless of the underlying protobuf
        // representation (grpc-js: Uint8Array; grpc-web: base64 string).
        // new Uint8Array(string) does NOT base64-decode, silently losing bytes.
        data = proto.getV0().getError().getData_asU8();
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
