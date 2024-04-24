const {
  v0: {
    GetMasternodeStatusRequest,
    GetMasternodeStatusResponse,
    CorePromiseClient,
  },
} = require('@dashevo/dapi-grpc');

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

    responseObject.proTxHash = Buffer.from(responseObject.proTxHash, 'base64');

    return responseObject;
  }

  return getMasternodeStatus;
}

module.exports = getMasternodeStatusFactory;
