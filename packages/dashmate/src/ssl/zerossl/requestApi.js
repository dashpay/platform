import * as errorDescriptions from './errors/errorDescriptions.js';

/**
 * Request the ZeroSSL API
 *
 * @param {string} url
 * @param {Object} options
 * @returns {Promise<Object>}
 */
export async function requestApi(url, options) {
  const response = await fetch(url, options);
  const data = await response.json();

  if (data.error) {
    const errorMessage = errorDescriptions[data.error.code];

    const error = new Error(errorMessage || data.error.type);

    Object.assign(error, data.error);

    throw error;
  }

  return data;
}
