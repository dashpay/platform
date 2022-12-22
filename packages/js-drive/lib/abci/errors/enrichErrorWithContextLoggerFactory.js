const { AsyncLocalStorage } = require('node:async_hooks');

/**
 * Add consensus logger to an error (factory)
 *
 * @return {enrichErrorWithContextLogger}
 */
function enrichErrorWithContextLoggerFactory() {
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
      const asyncLocalStorage = new AsyncLocalStorage();

      return asyncLocalStorage.run(new Map(), () => method(...args).catch((error) => {
        // eslint-disable-next-line no-param-reassign
        error.contextLogger = asyncLocalStorage.getStore().get('logger');

        return Promise.reject(error);
      }));
    }

    return methodHandler;
  }

  return enrichErrorWithContextLogger;
}

module.exports = enrichErrorWithContextLoggerFactory;
