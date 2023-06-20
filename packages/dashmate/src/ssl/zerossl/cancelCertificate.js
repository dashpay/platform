const fetch = require('node-fetch');
const errorDescriptions = require('./errors/errorDescriptions');

/**
 * Get ZeroSSL certificate
 *
 * @typedef {getCertificate}
 * @param {string} apiKey
 * @param {string} id
 * @return {Promise<Certificate>}
 */
async function cancelCertificate(apiKey, id) {
  const url = `https://api.zerossl.com/certificates/${id}/cancel?access_key=${apiKey}`;

  const requestOptions = {
    method: 'POST',
    headers: {
      'Content-Type': 'application/x-www-form-urlencoded',
    },
  };

  const response = await fetch(url, requestOptions);

  const data = await response.json();

  if (data.error) {
    const errorMessage = errorDescriptions[data.error.code];

    throw new Error(errorMessage || JSON.stringify(data.error));
  }
}

module.exports = cancelCertificate;
