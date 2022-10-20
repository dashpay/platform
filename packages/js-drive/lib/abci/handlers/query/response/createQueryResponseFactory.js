const {
  v0: {
    Proof,
    ResponseMetadata,
  },
} = require('@dashevo/dapi-grpc');

const UnavailableAbciError = require('../../../errors/UnavailableAbciError');

/**
 * @param {BlockExecutionContext} blockExecutionContext
 * @return {createQueryResponse}
 */
function createQueryResponseFactory(
  blockExecutionContext,
) {
  /**
   * @typedef {createQueryResponse}
   * @param {Function} ResponseClass
   * @param {boolean} [prove=false]
   */
  function createQueryResponse(ResponseClass, prove = false) {
    if (!blockExecutionContext) {
      throw new UnavailableAbciError('data is not available');
    }

    const blockHeight = blockExecutionContext.getHeight();
    const coreChainLockedHeight = blockExecutionContext.getCoreChainLockedHeight();

    const response = new ResponseClass();

    const metadata = new ResponseMetadata();
    metadata.setHeight(blockHeight);
    metadata.setCoreChainLockedHeight(coreChainLockedHeight);

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
