const {
  v0: {
    GetIdentityContractNonceResponse,
  },
} = require('@dashevo/dapi-grpc');

/**
 * @param {DriveClient} driveClient
 *
 * @returns {getIdentityContractNonceHandler}
 */
function getIdentityContractNonceHandlerFactory(driveClient) {
  /**
   * @typedef getIdentityContractNonceHandler
   *
   * @param {Object} call
   *
   * @return {Promise<GetIdentityContractNonceResponse>}
   */
  async function getIdentityContractNonceHandler(call) {
    const { request } = call;

    const identityContractNonceBuffer = await driveClient
      .fetchIdentityContractNonceRequest(request);

    return GetIdentityContractNonceResponse
      .deserializeBinary(identityContractNonceBuffer);
  }

  return getIdentityContractNonceHandler;
}

module.exports = getIdentityContractNonceHandlerFactory;
