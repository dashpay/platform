// const axios = require('axios');
const fetch = require('node-fetch');

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

  let url = `${protocol}://${host}${port && port !== 443 ? `:${port}` : ''}`;

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
    host,
    port,
    method,
    params,
    options,
  };

  let response;

  if (params.idq) {
    url = `${url}/?${params.idq}`;
  }

  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), options.timeout);

  try {
    const data = await fetch(
      url,
      { method: 'POST', body: payload, signal: controller.signal },
    );

    response = await data.json();

    // response = await axios.post(
    //   url,
    //   payload,
    //   { timeout: options.timeout },
    // );
  } catch (error) {
    if (error.response && error.response.status >= 500) {
      throw new WrongHttpCodeError(requestInfo, error.response.status, error.response.statusText);
    }

    throw error;
  } finally {
    clearTimeout(timeoutId);
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
