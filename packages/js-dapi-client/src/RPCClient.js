const axios = require('axios');

const defaultHost = '127.0.0.1';

/**
 * @param {string|object} url - rpc endpoint config
 * @param {string} [url.host]
 * @param {string|number} [url.port]
 * @param {string} method
 * @param {object} params
 * @param {object} options
 * @returns {Promise<*>}
 */
async function request(url, method, params, options = {}) {
  const destination = url.host
    ? `http://${url.host ? url.host : defaultHost}:${url.port ? url.port : ''}` : url;
  const payload = {
    jsonrpc: '2.0',
    method,
    params,
    id: 1,
  };
  const res = options.timeout
    ? await axios({
      method: 'post', url: destination, data: payload, timeout: options.timeout,
    }) : await axios.post(destination, payload);
  if (res.status !== 200) {
    throw new Error(res.statusMessage);
  }
  const { data } = res;
  if (data.error) {
    throw new Error(`DAPI RPC error: ${method}: ${data.error.message}`);
  }
  return data.result;
}

module.exports = {
  request,
};
