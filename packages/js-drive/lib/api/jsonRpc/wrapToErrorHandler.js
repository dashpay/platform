const { Server: { errors } } = require('jayson');
const createError = require('./createError');
const InvalidParamsError = require('../InvalidParamsError');

/**
 * Wrap API method to JSON RPC method handler
 *
 * @param {Function} method Api method
 * @return {Function}
 * @throws {Error}
 */
module.exports = function wrapToErrorHandler(method) {
  return async function apiMethodErrorHandler(params) {
    try {
      return await method(params);
    } catch (e) {
      if (e instanceof InvalidParamsError) {
        throw createError(errors.INVALID_PARAMS, e.message);
      }

      throw e;
    }
  };
};
