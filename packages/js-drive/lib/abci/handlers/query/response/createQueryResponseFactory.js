const {
  v0: {
    Proof,
    ResponseMetadata,
  },
} = require('@dashevo/dapi-grpc');

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
    const timeMs = latestBlockExecutionContext.getTimeMs();
    const version = latestBlockExecutionContext.getVersion();

    const response = new ResponseClass();

    const metadata = new ResponseMetadata();
    metadata.setHeight(blockHeight);
    metadata.setCoreChainLockedHeight(coreChainLockedHeight);
    metadata.setTimeMs(timeMs);
    metadata.setProtocolVersion(version.app);

    response.setMetadata(metadata);

    if (prove) {
      const {
        quorumHash,
        blockSignature: signature,
      } = latestBlockExecutionContext.getLastCommitInfo();

      const round = latestBlockExecutionContext.getRound();

      const proof = new Proof();

      proof.setQuorumHash(quorumHash);
      proof.setSignature(signature);
      proof.setRound(round);

      response.setProof(proof);
    }

    return response;
  }

  return createQueryResponse;
}

module.exports = createQueryResponseFactory;
