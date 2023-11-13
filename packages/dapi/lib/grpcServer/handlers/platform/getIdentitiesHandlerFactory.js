const {
  server: {
    error: {
      InvalidArgumentGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

const {
  v0: {
    GetIdentitiesResponse,
  },
} = require('@dashevo/dapi-grpc');

/**
 * @param {DriveClient} driveClient
 *
 * @return {getIdentitiesHandler}
 */
function getIdentitiesHandlerFactory(driveClient) {
  /**
   * @typedef getIdentitiesHandler
   *
   * @param {Object} call
   *
   * @return {Promise<GetIdentitiesResponse>}
   */
  async function getIdentitiesHandler(call) {
    const { request } = call;

    const identitiIds = request.getV0().getIdsList();

    if (identitiIds === null) {
      throw new InvalidArgumentGrpcError('identity ids are not specified');
    }

    const identitiesResponseBuffer = await driveClient.fetchIdentities(request);

    return GetIdentitiesResponse.deserializeBinary(identitiesResponseBuffer);
  }

  return getIdentitiesHandler;
}

module.exports = getIdentitiesHandlerFactory;
