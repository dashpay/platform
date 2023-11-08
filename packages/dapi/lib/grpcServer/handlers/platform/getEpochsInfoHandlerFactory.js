const {
  v0: {
    GetEpochsInfoResponse,
  },
} = require('@dashevo/dapi-grpc');

/**
 * @param {DriveClient} driveClient
 *
 * @returns {getEpochsInfoHandler}
 */
function getEpochsInfoHandlerFactory(driveClient) {
  /**
   * @typedef getEpochsInfoHandler
   *
   * @param {Object} call
   *
   * @return {Promise<GetEpochsInfoResponse>}
   */
  async function getEpochsInfoHandler(call) {
    const { request } = call;

    const epochsInfoBuffer = await driveClient
      .fetchEpochsInfo(request);

    return GetEpochsInfoResponse.deserializeBinary(epochsInfoBuffer);
  }

  return getEpochsInfoHandler;
}

module.exports = getEpochsInfoHandlerFactory;
