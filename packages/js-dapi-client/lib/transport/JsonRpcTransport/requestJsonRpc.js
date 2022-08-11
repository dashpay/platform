const axios = require('axios');
// const fetch = require('node-fetch').default;

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

  // const controller = new AbortController();
  // const timeoutId = setTimeout(() => controller.abort(), options.timeout);
  // const headers = { 'Content-type': 'application/json', 'Accept': 'application/json' };
  try {
    // response = await fetch(
    //   url,
    //   {
    //     method: 'POST', body: JSON.stringify(payload), signal: controller.signal, headers,
    //   },
    // );
    //
    // response = {
    //   data: await response.json(),
    //   status: response.status,
    //   statusMessage: response.statusMessage,
    // };

    // console.log(response);

    response = await axios.post(
      url,
      payload,
      { timeout: options.timeout },
    );
  } catch (error) {
    if (error.response && error.response.status >= 500) {
      throw new WrongHttpCodeError(requestInfo, error.response.status, error.response.statusText);
    }

    throw error;
  // } finally {
  //   clearTimeout(timeoutId);
  }

  if (response.status !== 200) {
    throw new WrongHttpCodeError(requestInfo, response.status, response.statusMessage);
  }

  const { data } = response;

  if (data.error) {
    console.log(data.error);
    throw new JsonRpcError(requestInfo, data.error);
  }

  return data.result;
}

module.exports = requestJsonRpc;
