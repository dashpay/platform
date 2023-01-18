/**
 *
 * @param {AsyncLocalStorage} abciAsyncLocalStorage
 * @return {createContextLogger}
 */
function createContextLoggerFactory(abciAsyncLocalStorage) {
  /**
   * @typedef {createContextLogger}
   * @param {Logger} logger
   * @param {Object} context
   * @return {Logger}
   */
  function createContextLogger(logger, context) {
    const contextLogger = logger.child(context);

    abciAsyncLocalStorage.getStore().set('logger', contextLogger);

    return contextLogger;
  }

  return createContextLogger;
}

module.exports = createContextLoggerFactory;
