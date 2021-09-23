/**
 * Add consensus logger to an error (factory)
 *
 * @param {blockExecutionContext} blockExecutionContext
 *
 * @return {enrichErrorWithConsensusLogger}
 */
function enrichErrorWithConsensusLoggerFactory(blockExecutionContext) {
  /**
   * Add consensus logger to an error
   *
   * @typedef enrichErrorWithConsensusLogger
   *
   * @param {Function} method
   *
   * @return {Function}
   */
  function enrichErrorWithConsensusLogger(method) {
    /**
     * @param {*} request
     */
    async function methodHandler(request) {
      try {
        return await method(request);
      } catch (e) {
        const { consensusLogger } = blockExecutionContext;

        if (consensusLogger) {
          e.consensusLogger = consensusLogger;
        }

        throw e;
      }
    }

    return methodHandler;
  }

  return enrichErrorWithConsensusLogger;
}

module.exports = enrichErrorWithConsensusLoggerFactory;
