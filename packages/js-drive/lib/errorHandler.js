/* eslint-disable no-console */

/**
 * Error handler
 *
 * @param {Error} e
 */
function errorHandler(e) {
  console.error(e);

  process.exit(1);
}

module.exports = errorHandler;
