const JsonRpcError = require('./errors/JsonRpcError');
const WrongHttpCodeError = require('./errors/WrongHttpCodeError');

// Lazily-created undici Agent that disables TLS verification, shared across
// all self-signed requests. A per-request Agent would leak its socket pool
// since nothing destroys it after the fetch completes.
let sharedSelfSignedAgent;
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

  // Self-signed HTTPS: Node 18+ built-in fetch is backed by undici, which
  // accepts a `dispatcher` for per-request TLS settings. Browsers can't
  // bypass TLS verification, so the flag is a no-op there. eval('require')
  // hides undici from bundler static analysis so it isn't pulled into
  // browser bundles.
  if (typeof process !== 'undefined'
    && process.versions != null
    && process.versions.node != null
    && protocol === 'https'
    && selfSigned) {
    if (!sharedSelfSignedAgent) {
      // eslint-disable-next-line no-eval, global-require
      const { Agent } = eval('require')('undici');
      sharedSelfSignedAgent = new Agent({ connect: { rejectUnauthorized: false } });
    }
    requestOptions.dispatcher = sharedSelfSignedAgent;
  }

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
