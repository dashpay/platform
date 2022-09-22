const axios = require('axios');

/**
 * List ZeroSSL certificates
 *
 * @typedef {listCertificates}
 * @param {string} apiKey
 * @return {Promise<Object>}
 */
async function listCertificates(apiKey) {
  const config = {
    method: 'get',
    url: `https://api.zerossl.com/certificates?access_key=${apiKey}`,
    headers: { },
  };

  const response = await axios(config);

  if (response.data.error) {
    throw new Error(response.data.error.type);
  }

  return response.data;
}

module.exports = listCertificates;
