const {
  v0: {
    GetMasternodeStatusRequest,
    GetMasternodeStatusResponse,
    CorePromiseClient,
  },
} = require('@dashevo/dapi-grpc');
const { base64ToBytes } = require('../../utils/bytes');

/**
 * @param {GrpcTransport} grpcTransport
 * @returns {getMasternodeStatus}
 */
function getMasternodeStatusFactory(grpcTransport) {
  /**
   * Get Core chain status
   * @typedef {getMasternodeStatus}
   * @param {DAPIClientOptions} [options]
   * @returns {Promise<object>}
   */
  async function getMasternodeStatus(options = {}) {
    const getMasternodeStatusRequest = new GetMasternodeStatusRequest();

    const response = await grpcTransport.request(
      CorePromiseClient,
      'getMasternodeStatus',
      getMasternodeStatusRequest,
      options,
    );

    const responseObject = response.toObject();

    // Respond with constant names instead of constant values

    responseObject.status = Object.keys(GetMasternodeStatusResponse.Status)
      .find((key) => GetMasternodeStatusResponse.Status[key] === responseObject.status);

    responseObject.proTxHash = base64ToBytes(responseObject.proTxHash);

    return responseObject;
  }

  return getMasternodeStatus;
}

module.exports = getMasternodeStatusFactory;
