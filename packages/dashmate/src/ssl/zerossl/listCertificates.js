const fetch = require('node-fetch');
const errorDescriptions = require('./errors/errorDescriptions');
const Certificate = require('./Certificate');

/**
 * List ZeroSSL certificates
 *
 * @typedef {listCertificates}
 * @param {string} apiKey
 * @param {String[]} [statuses] - possible values: draft, pending_validation, issued, cancelled,
 * revoked, expired.
 * @param {string} [search]
 * @return {Promise<Certificate[]>}
 */
async function listCertificates(apiKey, statuses = [], search = undefined) {
  let url = `https://api.zerossl.com/certificates?access_key=${apiKey}&limit=1000`;

  if (statuses.length > 0) {
    url += `&statuses=${statuses.join(',')}`;
  }

  if (search !== undefined) {
    url += `&search=${search}`;
  }

  const requestOptions = {
    method: 'GET',
    headers: {},
  };

  const response = await fetch(url, requestOptions);

  const data = await response.json();

  if (data.error) {
    const errorMessage = errorDescriptions[data.error.code];

    throw new Error(errorMessage || JSON.stringify(data.error));
  }

  return data.results.map((certificateData) => new Certificate(certificateData));
}

module.exports = listCertificates;
