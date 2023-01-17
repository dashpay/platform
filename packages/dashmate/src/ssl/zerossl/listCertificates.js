const fetch = require('node-fetch');

/**
 * List ZeroSSL certificates
 *
 * @typedef {listCertificates}
 * @param {string} apiKey
 * @return {Promise<Object>}
 */
async function listCertificates(apiKey) {
  const url = `https://api.zerossl.com/certificates?access_key=${apiKey}`;

  const requestOptions = {
    method: 'GET',
    headers: {},
  };

  const response = await fetch(url, requestOptions);

  const data = await response.json();

  if (data.error) {
    throw new Error(data.error.type);
  }

  return data;
}

module.exports = listCertificates;
