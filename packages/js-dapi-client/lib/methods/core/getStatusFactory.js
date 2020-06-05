const {
  GetStatusRequest,
  CorePromiseClient,
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

    return response.toObject();
  }

  return getStatus;
}

module.exports = getStatusFactory;
