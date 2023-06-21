const fetch = require('node-fetch');
const errorDescriptions = require('./errors/errorDescriptions');

/**
 * Download the certificate specified by id
 *
 * @typedef {downloadCertificate}
 * @param {string} id
 * @param {string} apiKey
 * @returns {Promise<string>}
 */
async function downloadCertificate(id, apiKey) {
  const url = `https://api.zerossl.com/certificates/${id}/download/return?access_key=${apiKey}`;

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

  return `${data['certificate.crt']}\n${data['ca_bundle.crt']}`;
}

module.exports = downloadCertificate;
