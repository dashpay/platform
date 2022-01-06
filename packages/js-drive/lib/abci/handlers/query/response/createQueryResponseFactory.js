const {
  v0: {
    Proof,
    ResponseMetadata,
  },
} = require('@dashevo/dapi-grpc');

/**
 * @param {BlockExecutionContextStack} blockExecutionContextStack
 * @return {createQueryResponse}
 */
function createQueryResponseFactory(
  blockExecutionContextStack,
) {
  /**
   * @typedef {createQueryResponse}
   * @param {Function} ResponseClass
   * @param {boolean} [prove=false]
   */
  function createQueryResponse(ResponseClass, prove = false) {
    const blockExecutionContext = blockExecutionContextStack.getFirst();
    const signedBlockExecutionContext = blockExecutionContextStack.getLast();

    const {
      height: signedBlockHeight,
      coreChainLockedHeight: signedCoreChainLockedHeight,
    } = signedBlockExecutionContext.getHeader();

    const response = new ResponseClass();

    const metadata = new ResponseMetadata();
    metadata.setHeight(signedBlockHeight);
    metadata.setCoreChainLockedHeight(signedCoreChainLockedHeight);

    response.setMetadata(metadata);

    if (prove) {
      const {
        quorumHash: signatureLlmqHash,
        stateSignature: signature,
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
