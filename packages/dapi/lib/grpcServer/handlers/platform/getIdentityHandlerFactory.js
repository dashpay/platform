const {
  server: {
    error: {
      InvalidArgumentGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

const {
  GetIdentityResponse,
} = require('@dashevo/dapi-grpc');

const AbciResponseError = require('../../../errors/AbciResponseError');

/**
 * @param {DriveStateRepository} driveStateRepository
 * @param {handleAbciResponseError} handleAbciResponseError
 *
 * @returns {getIdentityHandler}
 */
function getIdentityHandlerFactory(driveStateRepository, handleAbciResponseError) {
  /**
   * @typedef getIdentityHandler
   *
   * @param {Object} call
   *
   * @return {Promise<GetIdentityResponse>}
   */
  async function getIdentityHandler(call) {
    const { request } = call;

    const id = request.getId();

    if (!id) {
      throw new InvalidArgumentGrpcError('id is not specified');
    }

    let identityBuffer;
    try {
      identityBuffer = await driveStateRepository.fetchIdentity(id);
    } catch (e) {
      if (e instanceof AbciResponseError) {
        handleAbciResponseError(e);
      }
      throw e;
    }

    const response = new GetIdentityResponse();

    response.setIdentity(identityBuffer);

    return response;
  }

  return getIdentityHandler;
}

module.exports = getIdentityHandlerFactory;
