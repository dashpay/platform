const {
  v0: {
    MasternodeListRequest,
    CorePromiseClient,
  },
} = require('@dashevo/dapi-grpc');

/**
 * @param {GrpcTransport} grpcTransport
 * @returns {subscribeToMasternodeList}
 */
function subscribeToMasternodeListFactory(grpcTransport) {
  /**
   * @typedef {subscribeToMasternodeList}
   * @param {DAPIClientOptions & subscribeToMasternodeListOptions} [options]
   * @returns {
   *    EventEmitter|!grpc.web.ClientReadableStream<!MasternodeListResponse>
   * }
   */
  async function subscribeToMasternodeList(options = { }) {
    // eslint-disable-next-line no-param-reassign
    options = {
      // Override global timeout option
      // and timeout for this method by default
      timeout: undefined,
      ...options,
    };

    const request = new MasternodeListRequest();

    return grpcTransport.request(
      CorePromiseClient,
      'subscribeToMasternodeList',
      request,
      options,
    );
  }

  return subscribeToMasternodeList;
}

/**
 * @typedef {object} subscribeToMasternodeListOptions
 */

module.exports = subscribeToMasternodeListFactory;
