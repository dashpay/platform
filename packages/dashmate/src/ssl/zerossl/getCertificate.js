const fetch = require('node-fetch');
const errorDescriptions = require('./errors/errorDescriptions');

/**
 * Get ZeroSSL certificate
 *
 * @typedef {getCertificate}
 * @param {string} apiKey
 * @param {string} id
 * @return {Promise<Object>}
 */
async function getCertificate(apiKey, id) {
  const url = `https://api.zerossl.com/certificates/${id}?access_key=${apiKey}`;

  const requestOptions = {
    method: 'GET',
    headers: { },
  };

  const response = await fetch(url, requestOptions);

  const data = await response.json();

  if (data.error) {
    const errorMessage = errorDescriptions[data.error.code];

    throw new Error(errorMessage || JSON.stringify(data.error));
  }

  return data;
}

module.exports = getCertificate;
