const {
  v0: {
    GetStatusRequest,
    GetStatusResponse,
    CorePromiseClient,
  },
} = require('@dashevo/dapi-grpc');

/**
 * @param {GrpcTransport} grpcTransport
 * @returns {getStatus}
 */
function getStatusFactory(grpcTransport) {
  /**
   * Get Core chain status
   *
   * @typedef {getStatus}
   * @param {DAPIClientOptions} [options]
   * @returns {Promise<object>}
   */
  async function getStatus(options = {}) {
    const getStatusRequest = new GetStatusRequest();

    const response = await grpcTransport.request(
      CorePromiseClient,
      'getStatus',
      getStatusRequest,
      options,
    );

    const responseObject = response.toObject();

    responseObject.status = Object.keys(GetStatusResponse.Status)
      .find((key) => GetStatusResponse.Status[key] === responseObject.status);

    responseObject.masternode.status = Object.keys(GetStatusResponse.Masternode.Status)
      .find((key) => GetStatusResponse.Masternode.Status[key] === responseObject.masternode.status);

    return responseObject;
  }

  return getStatus;
}

module.exports = getStatusFactory;
