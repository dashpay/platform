const fetch = require('node-fetch');

const errorDescriptions = require('./errors/errorDescriptions');

/**
 * Request the ZeroSSL API
 *
 * @param {string} url
 * @param {Object} options
 * @returns {Promise<Object>}
 */
async function requestApi(url, options) {
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

module.exports = requestApi;
