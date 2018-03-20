const axios = require('axios');

const defaultHost = '127.0.0.1';

/**
 * @param {string|object} url - rpc endpoint config
 * @param {string} [url.host]
 * @param {string|number} [url.port]
 * @param method
 * @param params
 * @returns {Promise<*>}
 */
async function request(url, method, params) {
  let destination = url;
  if (typeof url !== 'string') {
    destination = `http://${url.host ? url.host : defaultHost}:${url.port ? url.port : ''}`;
  }
  const payload = {
    jsonrpc: '2.0',
    method,
    params,
    id: 1,
  };
  const res = await axios.post(destination, payload);
  if (res.status !== 200) {
    throw new Error(res.statusMessage);
  }
  const { data } = res;
  if (data.error) {
    throw new Error(`RPC error: ${method}: ${data.error.message}`);
  }
  return data.result;
}

module.exports = {
  request,
};
