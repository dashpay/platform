import errorDescriptions from './errors/errorDescriptions.js';
import ProviderUnreachableError from '../errors/ProviderUnreachableError.js';
import ProviderCredentialsRejectedError from '../errors/ProviderCredentialsRejectedError.js';

const INVALID_API_KEY_MESSAGE = 'Invalid ZeroSSL API key';
const INVALID_API_RESPONSE_MESSAGE = 'Invalid ZeroSSL API response';
const REDACTED_VALUE = '[REDACTED]';

/**
 * Redact the API key from a parsed ZeroSSL error without mutating the response.
 *
 * @param {*} value
 * @param {string} apiKey
 * @returns {*}
 */
function redactApiKey(value, apiKey) {
  if (typeof value === 'string') {
    return value.replaceAll(apiKey, REDACTED_VALUE);
  }

  if (Array.isArray(value)) {
    return value.map((item) => redactApiKey(item, apiKey));
  }

  if (value !== null && typeof value === 'object') {
    return Object.fromEntries(
      Object.entries(value).map(([key, item]) => [
        redactApiKey(key, apiKey),
        redactApiKey(item, apiKey),
      ]),
    );
  }

  return value;
}

/**
 * Build headers with one canonical ZeroSSL authorization value.
 *
 * @param {string} apiKey
 * @param {HeadersInit} sourceHeaders
 * @returns {Headers}
 */
function createHeaders(apiKey, sourceHeaders) {
  if (typeof apiKey !== 'string' || apiKey.length === 0 || apiKey.trim() !== apiKey) {
    throw new ProviderCredentialsRejectedError(INVALID_API_KEY_MESSAGE);
  }

  const authorization = `ApiKey ${apiKey}`;

  try {
    const headers = new Headers(sourceHeaders);
    headers.set('Authorization', authorization);

    if (headers.get('Authorization') !== authorization) {
      throw new ProviderCredentialsRejectedError(INVALID_API_KEY_MESSAGE);
    }

    return headers;
  } catch {
    throw new ProviderCredentialsRejectedError(INVALID_API_KEY_MESSAGE);
  }
}

/**
 * Request the ZeroSSL API
 *
 * @param {string} apiKey
 * @param {string} url
 * @param {Object} options
 * @returns {Promise<Object>}
 */
export default async function requestApi(apiKey, url, options) {
  const headers = createHeaders(apiKey, options.headers);
  const requestOptions = {
    ...options,
    headers,
  };

  // Wrapped where the request is made. `fetch failed` is the only account Node
  // gives of a transport failure, and recognising those words further down
  // would let any text carrying them - including a page this node's own
  // address served back - be read as this node's network failing.
  let response;

  try {
    response = await fetch(url, requestOptions);
  } catch (e) {
    throw new ProviderUnreachableError(e.message);
  }

  let data;
  try {
    data = await response.json();
  } catch {
    throw new ProviderUnreachableError(INVALID_API_RESPONSE_MESSAGE);
  }

  if (data.error) {
    const sanitizedError = redactApiKey(data.error, apiKey);
    const errorMessage = errorDescriptions[sanitizedError.code];

    const error = new Error(errorMessage || sanitizedError.type);

    Object.assign(error, sanitizedError);

    throw error;
  }

  return data;
}
