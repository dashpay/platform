/**
 * @return {subscribeToBlockHeadersWithChainLocksHandler}
 */
function subscribeToBlockHeadersWithChainLocksHandlerFactory() {
  /**
   * @typedef subscribeToBlockHeadersWithChainLocksHandler
   * @param {grpc.ServerWriteableStream<BlockHeadersWithChainLocksRequest>} call
   */
  async function subscribeToBlockHeadersWithChainLocksHandler(call) {
    call.end();
  }

  return subscribeToBlockHeadersWithChainLocksHandler;
}

module.exports = subscribeToBlockHeadersWithChainLocksHandlerFactory;
