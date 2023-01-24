const cbor = require('cbor');

const InvalidArgumentAbciError = require('../errors/InvalidArgumentAbciError');

/**
 * @param {Object} queryHandlerRouter
 * @param {Function} sanitizeUrl
 * @param {BaseLogger} logger
 * @param {createContextLogger} createContextLogger
 * @return {queryHandler}
 */
function queryHandlerFactory(queryHandlerRouter, sanitizeUrl, logger, createContextLogger) {
  /**
   * Query ABCI Handler
   *
   * @typedef queryHandler
   *
   * @param {RequestQuery} request
   * @return {Promise<Object>}
   */
  async function queryHandler(request) {
    const { path, data } = request;

    createContextLogger(logger, {
      abciMethod: 'query',
    });

    const route = queryHandlerRouter.find('GET', sanitizeUrl(path));

    if (!route) {
      throw new InvalidArgumentAbciError('Invalid path', { path });
    }

    const invalidDataMessage = 'Invalid data format: it should be cbor encoded object.';

    let encodedData = {};

    const decodeData = route.store && route.store.rawData === true;

    if (data.length > 0) {
      try {
        encodedData = decodeData ? Buffer.from(data) : cbor.decode(Buffer.from(data));
      } catch (e) {
        throw new InvalidArgumentAbciError(invalidDataMessage);
      }

      if (encodedData === null || typeof encodedData !== 'object') {
        throw new InvalidArgumentAbciError(invalidDataMessage);
      }
    }

    return route.handler(route.params, encodedData, request);
  }

  return queryHandler;
}

module.exports = queryHandlerFactory;
