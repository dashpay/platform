const axios = require('axios');
const qs = require('qs');

/**
 * Verify the domain/ip specified by certificate id
 *
 * @typedef {verifyDomain}
 * @param {string} id
 * @param {string} apiKey
 * @return {Promise<Object>}
 */
async function verifyDomain(id, apiKey) {
  const data = qs.stringify({
    validation_method: 'HTTP_CSR_HASH',
  });

  const request = {
    method: 'post',
    url: `https://api.zerossl.com/certificates/${id}/challenges?access_key=${apiKey}`,
    headers: {
      'Content-Type': 'application/x-www-form-urlencoded',
    },
    data,
  };

  const response = await axios(request);

  if (response.data.error) {
    throw new Error(response.data.error.type);
  }

  return response.data;
}

module.exports = verifyDomain;
