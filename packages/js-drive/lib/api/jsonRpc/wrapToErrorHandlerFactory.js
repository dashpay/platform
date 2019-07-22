const { Server: { errors } } = require('jayson');
const createError = require('./createError');
const InvalidParamsError = require('../InvalidParamsError');

/**
 *
 * @param {Logger} logger
 * @returns wrapToErrorHandler
 */
module.exports = function wrapInErrorHandlerFactory(logger) {
  /**
   * Wrap API method to JSON RPC method handler
   *
   * @typedef wrapToErrorHandler
   * @param {Function} method Api method
   * @return {Function}
   * @throws {Error}
   */
  function wrapToErrorHandler(method) {
    /**
     *
     * @param {Object} params
     * @returns {Promise<void>}
     */
    async function apiMethodErrorHandler(params) {
      try {
        return await method(params);
      } catch (e) {
        if (e instanceof InvalidParamsError) {
          throw createError(errors.INVALID_PARAMS, e.message, e.data);
        }

        logger.error(`Error in ${method.name} API method`, { method: method.name, params }, e);

        throw e;
      }
    }

    return apiMethodErrorHandler;
  }

  return wrapToErrorHandler;
};
