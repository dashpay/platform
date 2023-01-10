/**
 * Add consensus logger to an error (factory)
 *
 * @param {AsyncLocalStorage} abciAsyncLocalStorage
 * @return {enrichErrorWithContextLogger}
 */
function enrichErrorWithContextLoggerFactory(abciAsyncLocalStorage) {
  /**
   * Add consensus logger to an error
   *
   * @typedef enrichErrorWithContextLogger
   *
   * @param {Function} method
   *
   * @return {Function}
   */
  function enrichErrorWithContextLogger(method) {
    /**
     * @param {*[]} args
     */
    async function methodHandler(...args) {

      return abciAsyncLocalStorage.run(
        new Map(),
        () => method(...args).catch((error) => {
          // eslint-disable-next-line no-param-reassign
          error.contextLogger = abciAsyncLocalStorage.getStore().get('logger');

          return Promise.reject(error);
        }),
      );
    }

    return methodHandler;
  }

  return enrichErrorWithContextLogger;
}

module.exports = enrichErrorWithContextLoggerFactory;
