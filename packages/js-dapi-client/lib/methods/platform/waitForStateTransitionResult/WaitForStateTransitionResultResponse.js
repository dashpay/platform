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

    if (proto.getProof()) {
      proof = new Proof({
        rootTreeProof: Buffer.from(proto.getProof().getRootTreeProof()),
        storeTreeProof: Buffer.from(proto.getProof().getStoreTreeProof()),
        signatureLLMQHash: Buffer.from(proto.getProof().getSignatureLlmqHash()),
        signature: Buffer.from(proto.getProof().getSignature()),
      });
    }

    if (proto.getError()) {
      let data;
      const rawData = proto.getError().getData();
      if (rawData) {
        data = cbor.decode(Buffer.from(rawData));
      }

      error = new ErrorResult(
        proto.getError().getCode(),
        proto.getError().getMessage(),
        data,
      );
    }

    const metadata = proto.getMetadata() ? new Metadata(proto.getMetadata().toObject()) : null;

    return new WaitForStateTransitionResultResponse(
      metadata,
      proof,
      error,
    );
  }
}

module.exports = WaitForStateTransitionResultResponse;
