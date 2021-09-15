const cbor = require('cbor');

const InvalidArgumentGrpcError = require('@dashevo/grpc-common/lib/server/error/InvalidArgumentGrpcError');

/**
 * @param {Object} queryHandlerRouter
 * @param {Function} sanitizeUrl
 * @return {queryHandler}
 */
function queryHandlerFactory(queryHandlerRouter, sanitizeUrl) {
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

    const route = queryHandlerRouter.find('GET', sanitizeUrl(path));

    if (!route) {
      throw new InvalidArgumentGrpcError('Invalid path', { path });
    }

    const invalidDataMessage = 'Invalid data format: it should be cbor encoded object.';

    let encodedData = {};

    const decodeData = route.store && route.store.rawData === true;

    if (data.length > 0) {
      try {
        encodedData = decodeData ? Buffer.from(data) : cbor.decode(Buffer.from(data));
      } catch (e) {
        throw new InvalidArgumentGrpcError(invalidDataMessage);
      }

      if (encodedData === null || typeof encodedData !== 'object') {
        throw new InvalidArgumentGrpcError(invalidDataMessage);
      }
    }

    return route.handler(route.params, encodedData, request);
  }

  return queryHandler;
}

module.exports = queryHandlerFactory;
