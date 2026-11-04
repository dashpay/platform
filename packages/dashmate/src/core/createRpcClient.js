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
export default function createRpcClient(config = {}) {
  // eslint-disable-next-line no-param-reassign
  config = {
    protocol: 'http',
    user: 'dashrpc',
    pass: 'password',
    host: '127.0.0.1',
    port: 20002,
    ...config,
  };

  const rpcClient = new RpcClient(config);

  /**
   * Call a Core RPC method with its params sent exactly as given
   *
   * dashd-rpc converts leading params to the types in its callspec, which sends
   * `protx register` collateralIndex as a string. Core v24 rejects that.
   *
   * @param {string} method
   * @param {Array} params
   * @param {Object} [options]
   * @param {string} [options.wallet]
   * @return {Promise<{ result: * }>}
   */
  rpcClient.callWithParams = async (method, params, { wallet } = {}) => {
    const path = wallet ? `/wallet/${encodeURIComponent(wallet)}` : '/';
    const auth = Buffer.from(`${config.user}:${config.pass}`).toString('base64');

    const response = await fetch(`${config.protocol}://${config.host}:${config.port}${path}`, {
      method: 'POST',
      headers: {
        Authorization: `Basic ${auth}`,
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ id: 1, method, params }),
      signal: rpcClient.timeout ? AbortSignal.timeout(rpcClient.timeout) : undefined,
    });

    const text = await response.text();

    let body;
    try {
      body = JSON.parse(text);
    } catch {
      throw new Error(`Dash JSON-RPC: HTTP ${response.status}: ${text}`);
    }

    if (body.error) {
      const error = new Error(body.error.message);
      error.code = body.error.code;

      throw error;
    }

    return body;
  };

  return rpcClient;
}
