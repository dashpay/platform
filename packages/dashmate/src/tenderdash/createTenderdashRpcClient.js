import {client} from 'jayson/promise'

/**
 * Create Tenderdash RPC client
 *
 * @param {Object} [options]
 * @param {string} [options.host]
 * @param {number} [options.port]
 */
export function createTenderdashRpcClient({ host, port } = {}) {
  return client.http({
    host: host || '127.0.0.1',
    port: port || 26657,
  });
}
