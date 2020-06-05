const axios = require('axios');

const JsonRpcError = require('./errors/JsonRpcError');
const WrongHttpCodeError = require('./errors/WrongHttpCodeError');

/**
 * @typedef {requestJsonRpc}
 * @param {string} host
 * @param {number} port
 * @param {string} method
 * @param {object} params
 * @param {object} [options]
 * @returns {Promise<*>}
 */
async function requestJsonRpc(host, port, method, params, options = {}) {
  const protocol = port === 443 ? 'https' : 'http';

  const url = `${protocol}://${host}${port && port !== 443 ? `:${port}` : ''}`;

  const payload = {
    jsonrpc: '2.0',
    method,
    params,
    id: 1,
  };

  const postOptions = {};
  if (options.timeout !== undefined) {
    postOptions.timeout = options.timeout;
  }

  const response = await axios.post(
    url,
    payload,
    { timeout: options.timeout },
  );

  const requestInfo = {
    host,
    port,
    method,
    params,
    options,
  };

  if (response.status !== 200) {
    throw new WrongHttpCodeError(requestInfo, response.status, response.statusMessage);
  }

  const { data } = response;

  if (data.error) {
    throw new JsonRpcError(requestInfo, data.error);
  }

  return data.result;
}

module.exports = requestJsonRpc;
