const {
  LastUserStateTransitionHashResponse,
} = require('@dashevo/dapi-grpc');

const InvalidArgumentGrpcError = require('../../error/InvalidArgumentGrpcError');

/**
 * @param {RpcClient} coreAPI
 * @returns {getLastUserStateTransitionHashHandler}
 */
function getLastUserStateTransitionHashHandlerFactory(coreAPI) {
  /**
   * @typedef getLastUserStateTransitionHashHandler
   * @param {Object} call
   */
  async function getLastUserStateTransitionHashHandler(call) {
    const { request } = call;

    const userIdBuffer = request.getUserId_asU8();

    if (!userIdBuffer || userIdBuffer.length === 0) {
      throw new InvalidArgumentGrpcError('userId is not specified');
    }

    const userId = Buffer.from(userIdBuffer)
      .toString('hex');

    let user;
    try {
      user = await coreAPI.getUser(userId);
    } catch (e) {
      throw new InvalidArgumentGrpcError(`Could not retrieve user by id ${userId}. Reason: ${e.message}`);
    }

    const response = new LastUserStateTransitionHashResponse();

    if (Array.isArray(user.subtx) && user.subtx.length > 0) {
      const stateTransitionHash = Buffer.from(
        user.subtx[user.subtx.length - 1],
        'hex',
      );

      response.setStateTransitionHash(stateTransitionHash);
    }

    return response;
  }

  return getLastUserStateTransitionHashHandler;
}

module.exports = getLastUserStateTransitionHashHandlerFactory;
