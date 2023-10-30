const {
  v0: {
    GetEpochsInfoResponse,
  },
} = require('@dashevo/dapi-grpc');

/**
 * @param {DriveClient} driveClient
 *
 * @returns {getEpochInfosHandler}
 */
function getEpochsInfoHandlerFactory(driveClient) {
  /**
   * @typedef getEpochInfosHandler
   *
   * @param {Object} call
   *
   * @return {Promise<GetEpochsInfoResponse>}
   */
  async function getEpochInfosHandler(call) {
    const { request } = call;

    const epochInfosBuffer = await driveClient
      .fetchEpochsInfo(request);

    return GetEpochsInfoResponse.deserializeBinary(epochInfosBuffer);
  }

  return getEpochInfosHandler;
}

module.exports = getEpochsInfoHandlerFactory;
