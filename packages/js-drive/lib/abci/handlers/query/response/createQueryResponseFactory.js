const {
  v0: {
    Proof,
    ResponseMetadata,
  },
} = require('@dashevo/dapi-grpc');

/**
 * @param {BlockExecutionContext} blockExecutionContext
 * @param {BlockExecutionContext} previousBlockExecutionContext
 * @return {createQueryResponse}
 */
function createQueryResponseFactory(
  blockExecutionContext,
  previousBlockExecutionContext,
) {
  /**
   * @typedef {createQueryResponse}
   * @param {Function} ResponseClass
   * @param {boolean} [prove=false]
   */
  function createQueryResponse(ResponseClass, prove = false) {
    const {
      height: previousBlockHeight,
      coreChainLockedHeight: previousCoreChainLockedHeight,
    } = previousBlockExecutionContext.getHeader();

    const response = new ResponseClass();

    const metadata = new ResponseMetadata();
    metadata.setHeight(previousBlockHeight);
    metadata.setCoreChainLockedHeight(previousCoreChainLockedHeight);

    response.setMetadata(metadata);

    if (prove) {
      const {
        quorumHash: signatureLlmqHash,
        signature,
      } = blockExecutionContext.getLastCommitInfo();

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
