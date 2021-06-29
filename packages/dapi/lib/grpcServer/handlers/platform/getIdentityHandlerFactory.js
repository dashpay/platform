const {
  server: {
    error: {
      InvalidArgumentGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

const {
  v0: {
    GetIdentityResponse,
  },
} = require('@dashevo/dapi-grpc');

const AbciResponseError = require('../../../errors/AbciResponseError');

/**
 * @param {DriveClient} driveClient
 * @param {handleAbciResponseError} handleAbciResponseError
 *
 * @returns {getIdentityHandler}
 */
function getIdentityHandlerFactory(driveClient, handleAbciResponseError) {
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

    const prove = request.getProve();

    let identityResponseBuffer;

    try {
      identityResponseBuffer = await driveClient
        .fetchIdentity(id, prove);
    } catch (e) {
      if (e instanceof AbciResponseError) {
        handleAbciResponseError(e);
      }
      throw e;
    }

    return GetIdentityResponse.deserializeBinary(identityResponseBuffer);
  }

  return getIdentityHandler;
}

module.exports = getIdentityHandlerFactory;
