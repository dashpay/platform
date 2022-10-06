const axios = require('axios');

/**
 * Get ZeroSSL certificate
 *
 * @typedef {getCertificate}
 * @param {string} apiKey
 * @param {string} id
 * @return {Promise<Object>}
 */
async function getCertificate(apiKey, id) {
  const config = {
    method: 'get',
    url: `https://api.zerossl.com/certificates/${id}?access_key=${apiKey}`,
    headers: { },
  };

  const response = await axios(config);

  if (response.data.error) {
    throw new Error(response.data.error.type);
  }

  return response.data;
}

module.exports = getCertificate;
