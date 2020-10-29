const { client: JsonRpcClient } = require('jayson/promise');

/**
 * Create Tenderdash RPC client
 *
 * @param {Object} [options]
 * @param {string} [options.host]
 * @param {number} [options.port]
 */
function createTenderdashRpcClient({ host, port } = {}) {
  return JsonRpcClient.http({
    host: host || '127.0.0.1',
    port: port || 26657,
  });
}

module.exports = createTenderdashRpcClient;
