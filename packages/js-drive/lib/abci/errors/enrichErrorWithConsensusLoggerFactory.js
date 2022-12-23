/**
 * Add consensus logger to an error (factory)
 *
 * @param {BlockExecutionContext} latestBlockExecutionContext
 *
 * @return {enrichErrorWithConsensusLogger}
 */
function enrichErrorWithConsensusLoggerFactory(latestBlockExecutionContext) {
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
     * @param {*[]} args
     */
    async function methodHandler(...args) {
      try {
        return await method(...args);
      } catch (e) {
        const { consensusLogger } = latestBlockExecutionContext;

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
