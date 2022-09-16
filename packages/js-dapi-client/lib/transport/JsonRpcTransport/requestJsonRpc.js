const axios = require('axios');
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

  const postOptions = {};
  if (options.timeout !== undefined) {
    postOptions.timeout = options.timeout;
  }

  const requestInfo = {
    protocol,
    host,
    port,
    selfSigned,
    method,
    params,
    options,
  };

  let response;

  const config = { timeout: options.timeout };
  // For NodeJS Client
  if (typeof process !== 'undefined'
    && process.versions != null
    && process.versions.node != null
    && protocol === 'https'
    && selfSigned) {
    config.httpsAgent = new https.Agent({
      rejectUnauthorized: false,
    });
  }

  try {
    response = await axios.post(
      url,
      payload,
      config,
    );
  } catch (error) {
    if (error.response && error.response.status >= 500) {
      throw new WrongHttpCodeError(requestInfo, error.response.status, error.response.statusText);
    }

    throw error;
  }

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
