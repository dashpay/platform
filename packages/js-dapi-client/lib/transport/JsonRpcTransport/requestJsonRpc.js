const https = require('https');
const JsonRpcError = require('./errors/JsonRpcError');
const WrongHttpCodeError = require('./errors/WrongHttpCodeError');
/**
 * @typedef {requestJsonRpc}
 * @param {string} protocol
 * @param {string} host
 * @param {number} port
 * @param {boolean} selfSigned
 * @param {string} method
 * @param {object} params
 * @param {object} [options]
 * @returns {Promise<*>}
 */
async function requestJsonRpc(protocol, host, port, selfSigned, method, params, options = {}) {
  const url = `${protocol}://${host}${port && port !== 443 ? `:${port}` : ''}`;

  const payload = {
    jsonrpc: '2.0',
    method,
    params,
    id: 1,
  };

  const requestInfo = {
    protocol,
    host,
    port,
    selfSigned,
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

  // For NodeJS Client
  if (typeof process !== 'undefined'
    && process.versions != null
    && process.versions.node != null
    && protocol === 'https'
    && selfSigned) {
    requestOptions.agent = new https.Agent({
      rejectUnauthorized: false,
    });
  }
  // eslint-disable-next-line
  const response = await fetch(url, requestOptions);

  if (typeof requestTimeoutId !== 'undefined') {
    clearTimeout(requestTimeoutId);
  }

  if (!response.ok) {
    throw new WrongHttpCodeError(requestInfo, response.status, response.statusText);
  }

  const data = await response.json();

  if (data.error) {
    throw new JsonRpcError(requestInfo, data.error);
  }

  return data.result;
}

module.exports = requestJsonRpc;
