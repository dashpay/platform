const {
  server: {
    error: {
      UnimplementedGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

const logger = require('../../../logger');

/**
 * @param {string} name
 * @returns {broadcastStateTransitionHandler}
 */
function unimplementedHandlerFactory(name) {
  /**
   * @typedef broadcastStateTransitionHandler
   *
   * @return {Promise<BroadcastStateTransitionResponse>}
   */
  async function broadcastStateTransitionHandler() {
    logger.error(`unimplemented endpoint '${name}' called`);

    throw new UnimplementedGrpcError('the endpoint is not implemented in DAPI');
  }

  return broadcastStateTransitionHandler;
}

module.exports = unimplementedHandlerFactory;
