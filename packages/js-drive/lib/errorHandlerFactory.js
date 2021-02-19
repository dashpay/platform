const printErrorFace = require('./util/printErrorFace');

/**
 * @param {BaseLogger} logger
 * @param {AwilixContainer} container
 * @param {closeAbciServer} closeAbciServer
 */
function errorHandlerFactory(logger, container, closeAbciServer) {
  let isCalledAlready = false;
  const errors = [];

  /**
   * Error handler
   *
   * @param {Error} error
   */
  async function errorHandler(error) {
    // Collect all thrown errors
    errors.push(error);

    // Gracefully shutdown only once
    if (isCalledAlready) {
      return;
    }

    isCalledAlready = true;

    try {
      try {
        // Close all ABCI server connections
        await closeAbciServer();

        // Add further code to the end of event loop (the same as process.nextTick)
        await Promise.resolve();

        // eslint-disable-next-line no-console
        console.log(printErrorFace());

        errors.forEach((e) => {
          (error.consensusLogger || logger).fatal({ err: e }, e.message);
        });
      } finally {
        await container.dispose();
      }
    } catch (e) {
      // eslint-disable-next-line no-console
      console.error(e);
    } finally {
      process.exit(1);
    }
  }

  return errorHandler;
}

module.exports = errorHandlerFactory;
