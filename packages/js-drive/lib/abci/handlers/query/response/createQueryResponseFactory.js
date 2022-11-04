const {
  v0: {
    Proof,
    ResponseMetadata,
  },
} = require('@dashevo/dapi-grpc');
const { Timestamp } = require('google-protobuf/google/protobuf/timestamp_pb');

const UnavailableAbciError = require('../../../errors/UnavailableAbciError');

/**
 * @param {BlockExecutionContext} latestBlockExecutionContext
 * @return {createQueryResponse}
 */
function createQueryResponseFactory(
  latestBlockExecutionContext,
) {
  /**
   * @typedef {createQueryResponse}
   * @param {Function} ResponseClass
   * @param {boolean} [prove=false]
   */
  function createQueryResponse(ResponseClass, prove = false) {
    if (latestBlockExecutionContext.isEmpty()) {
      throw new UnavailableAbciError('data is not available');
    }

    const blockHeight = latestBlockExecutionContext.getHeight();
    const coreChainLockedHeight = latestBlockExecutionContext.getCoreChainLockedHeight();
    const time = latestBlockExecutionContext.getTime();
    const version = latestBlockExecutionContext.getVersion();

    const protobufTime = new Timestamp();
    protobufTime.setSeconds(time.seconds);
    protobufTime.setNanos(time.nanos);

    const {
      blockSignature,
    } = latestBlockExecutionContext.getLastCommitInfo();

    const response = new ResponseClass();

    const metadata = new ResponseMetadata();
    metadata.setHeight(blockHeight);
    metadata.setCoreChainLockedHeight(coreChainLockedHeight);
    metadata.setSignature(blockSignature);
    metadata.setTime(protobufTime);
    metadata.setProtocolVersion(version.app);

    response.setMetadata(metadata);

    if (prove) {
      const {
        quorumHash: signatureLlmqHash,
        stateSignature: signature,
      } = latestBlockExecutionContext.getLastCommitInfo();

      const proof = new Proof();

      proof.setSignatureLlmqHash(signatureLlmqHash);
      proof.setSignature(signature);

      response.setProof(proof);
    }

    return response;
  }

  return createQueryResponse;
}

module.exports = createQueryResponseFactory;
