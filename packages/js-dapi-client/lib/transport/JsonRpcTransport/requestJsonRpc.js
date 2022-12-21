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

  const requestInfo = {
    host,
    port,
    method,
    params,
    options,
  };

  const requestOptions = {
    method: 'POST',
    body: JSON.stringify(payload),
    headers: {
      'Content-Type': 'application/json',
    },
  };

  let requestTimeoutId;
  if (options.timeout) {
    const controller = new AbortController();
    requestTimeoutId = setTimeout(() => controller.abort(), options.timeout);
    Object.assign(requestOptions, { signal: controller.signal });
  }

  // eslint-disable-next-line
  const response = await fetch(url, requestOptions);

  if (typeof requestTimeoutId !== 'undefined') {
    clearTimeout(requestTimeoutId);
  }
  const data = await response.json();

  if (!response.ok) {
    throw new WrongHttpCodeError(requestInfo, response.status, response.statusText);
  }

  if (data.error) {
    throw new JsonRpcError(requestInfo, data.error);
  }

  return data.result;
}

module.exports = requestJsonRpc;
