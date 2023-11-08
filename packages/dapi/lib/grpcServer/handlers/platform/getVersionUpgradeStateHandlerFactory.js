const {
  v0: {
    GetVersionUpgradeStateResponse,
  },
} = require('@dashevo/dapi-grpc');

/**
 * @param {DriveClient} driveClient
 *
 * @returns {getVersionUpgradeStateHandler}
 */
function getVersionUpgradeStateHandlerFactory(driveClient) {
  /**
   * @typedef getVersionUpgradeStateHandler
   *
   * @param {Object} call
   *
   * @return {Promise<GetVersionUpgradeStateResponse>}
   */
  async function getVersionUpgradeStateHandler(call) {
    const { request } = call;

    const versionUpgradeStateBuffer = await driveClient
      .fetchVersionUpgradeState(request);

    return GetVersionUpgradeStateResponse
      .deserializeBinary(versionUpgradeStateBuffer);
  }

  return getVersionUpgradeStateHandler;
}

module.exports = getVersionUpgradeStateHandlerFactory;
