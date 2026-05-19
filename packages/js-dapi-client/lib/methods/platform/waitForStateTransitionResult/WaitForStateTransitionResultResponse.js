import AbstractResponse from '../response/AbstractResponse.js';
import Metadata from '../response/Metadata.js';
import Proof from '../response/Proof.js';
import ErrorResult from './ErrorResult.js';

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
        data = new Uint8Array(proto.getV0().getError().getData());
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

export default WaitForStateTransitionResultResponse;
