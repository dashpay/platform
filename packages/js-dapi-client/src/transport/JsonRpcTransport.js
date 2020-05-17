const rpcClient = require('../RPCClient');

class JsonRpcTransport {
  /**
   * @param {MNDiscovery} mnDiscovery
   * @param {number} dapiPort
   */
  constructor(mnDiscovery, dapiPort) {
    this.mnDiscovery = mnDiscovery;
    this.dapiPort = dapiPort;
  }

  /**
   * Make request to a random MN (with retries)
   *
   * @param {string} method
   * @param {Object} params
   * @param {Object} options
   * @param {number} [options.retriesCount=3]
   * @param {string[]} [options.excludedIps=[]]
   * @param {Object} [options.client={}]
   * @param {number} [options.client.timeout]
   *
   * @returns {Promise<*|undefined>}
   */
  async makeRequest(method, params, options = { retriesCount: 3, excludedIps: [] }) {
    const retriesCount = options.retriesCount != null ? options.retriesCount : 3;
    const excludedIps = options.excludedIps || [];
    const clientOptions = options.client || {};

    let urlToConnect;
    try {
      urlToConnect = await this.getJsonRpcUrl(excludedIps);

      const result = await rpcClient.request(
        urlToConnect, method, params, { timeout: 10000, ...clientOptions },
      );

      return result;
    } catch (e) {
      if (e.code !== 'ECONNABORTED' && e.code !== 'ECONNREFUSED') {
        throw e;
      }

      if (retriesCount > 0) {
        const { host: currentMasternodeIp } = urlToConnect;

        return this.makeRequest(
          method, params, {
            ...options,
            retriesCount: retriesCount - 1,
            excludedIps: [currentMasternodeIp, ...excludedIps],
          },
        );
      }

      throw new Error('max retries to connect to DAPI node reached');
    }
  }

  /**
   * @private
   *
   * Get JSON RPC url object
   *
   * @param {string[]} [excludedIps]
   *
   * @returns {Promise<Object>}
   */
  async getJsonRpcUrl(excludedIps = []) {
    const randomMasternode = await this.mnDiscovery.getRandomMasternode(excludedIps);

    return {
      host: randomMasternode.getIp(),
      port: this.dapiPort,
    };
  }
}

module.exports = JsonRpcTransport;
