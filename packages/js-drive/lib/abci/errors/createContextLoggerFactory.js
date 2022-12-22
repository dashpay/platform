function createContextLoggerFactory(abciAsyncLocalStorage) {
  function createContextLogger(logger, context) {
    const contextLogger = logger.child(context);

    abciAsyncLocalStorage.getStore().set('logger', contextLogger);

    return contextLogger;
  }

  return createContextLogger;
}

module.exports = createContextLoggerFactory;
