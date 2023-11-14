import RpcClient from '@dashevo/dashd-rpc/promise.js';

/**
 * Create Core JSON RPC Client
 *
 * @typedef createRpcClient
 * @param {Object} [config]
 * @param {string} [config.protocol=http]
 * @param {string} [config.user=dashrpc]
 * @param {string} [config.pass=password]
 * @param {string} [config.host=127.0.0.1]
 * @param {number} [config.port=20002]
 * @return {RpcClient|PromisifyModule}
 */
export function createRpcClient(config = {}) {
  // eslint-disable-next-line no-param-reassign
  config = {
    protocol: 'http',
    user: 'dashrpc',
    pass: 'password',
    host: '127.0.0.1',
    port: 20002,
    ...config,
  };

  return new RpcClient(config);
}
