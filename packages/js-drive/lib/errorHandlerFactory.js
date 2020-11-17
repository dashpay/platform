/* eslint-disable no-console */

/**
 * @param {AwilixContainer} container
 */
function errorHandlerFactory(container) {
  /**
   * Error handler
   *
   * @param {Error} e
   */
  async function errorHandler(e) {
    await container.dispose();

    console.error(e);

    process.exit(1);
  }

  return errorHandler;
}

module.exports = errorHandlerFactory;
