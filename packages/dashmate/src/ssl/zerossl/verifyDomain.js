const axios = require('axios');

/**
 * Verify the domain/ip specified by certificate id
 *
 * @param {string} id
 * @param {Config} config
 */
async function verifyDomain(id, config) {
  const data = new URLSearchParams({
    validation_method: 'HTTP_CSR_HASH',
  }).toString();

  const request = {
    method: 'post',
    url: `https://api.zerossl.com/certificates/${id}/challenges?access_key=${config.get(
      'platform.dapi.nginx.ssl.zerossl.apikey'
    )}`,
    headers: {
      'Content-Type': 'application/x-www-form-urlencoded',
    },
    data,
  };

  const response = await axios(request).catch((error) => {
    throw new Error(error);
  });
  return response;
}

module.exports = verifyDomain;
