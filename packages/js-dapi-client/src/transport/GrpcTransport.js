// eslint-disable-next-line import/no-extraneous-dependencies
const GrpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');

class GrpcTransport {
  /**
   * @param {MNDiscovery} mnDiscovery
   * @param {number} dapiPort
   * @param {number} grpcNativePort
   * @param {
   *   CorePromiseClient|PlatformPromiseClient|TransactionsFilterStreamPromiseClient
   * } ClientClass
   */
  constructor(mnDiscovery, dapiPort, grpcNativePort, ClientClass) {
    this.mnDiscovery = mnDiscovery;
    this.dapiPort = dapiPort;
    this.grpcNativePort = grpcNativePort;
    this.ClientClass = ClientClass;
  }

  /**
   * Make request to a random MN (with retries)
   *
   * @param {string} method
   * @param {Object} request
   * @param {Object} options
   * @param {number} [options.retriesCount=3]
   * @param {string[]} [options.excludedIps=[]]
   * @param {Object} [options.client={}]
   * @param {number} [options.client.timeout]
   *
   * @returns {Promise<*|undefined>}
   */
  async makeRequest(method, request, options = { retriesCount: 3, excludedIps: [] }) {
    const retriesCount = options.retriesCount != null ? options.retriesCount : 3;
    const excludedIps = options.excludedIps || [];

    let urlToConnect;
    try {
      urlToConnect = await this.getGrpcUrl(excludedIps);

      const client = new this.ClientClass(urlToConnect);

      const result = await client[method](request);

      return result;
    } catch (e) {
      if (e.code !== GrpcErrorCodes.DEADLINE_EXCEEDED
            && e.code !== GrpcErrorCodes.UNAVAILABLE
            && e.code !== GrpcErrorCodes.INTERNAL) {
        throw e;
      }

      if (retriesCount > 0) {
        const currentMasternodeIp = urlToConnect.split(':')[0];

        return this.makeRequest(
          method, request, {
            ...options,
            retriesCount: retriesCount - 1,
            excludedIps: [currentMasternodeIp, ...excludedIps],
          },
        );
      }

      throw e;
    }
  }

  /**
   * @private
   *
   * Get gRPC url string
   *
   * @param {string[]} [excludedIps]
   *
   * @returns {Promise<string>}
   */
  async getGrpcUrl(excludedIps = []) {
    const randomMasternode = await this.mnDiscovery.getRandomMasternode(excludedIps);

    if (typeof process !== 'undefined'
      && process.versions != null
      && process.versions.node != null) {
      return `${randomMasternode.getIp()}:${this.grpcNativePort}`;
    }

    return `http://${randomMasternode.getIp()}:${this.dapiPort}`;
  }
}

module.exports = GrpcTransport;
