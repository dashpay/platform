const axios = require('axios');
const qs = require('qs');

/**
 * Verify the domain/ip specified by certificate id
 *
 * @param {string} id
 * @param {Config} config
 */
async function verifyDomain(id, config) {
  const data = qs.stringify({
    validation_method: 'HTTP_CSR_HASH',
  });

  const request = {
    method: 'post',
    url: `https://api.zerossl.com/certificates/${id}/challenges?access_key=${config.get('platform.dapi.nginx.ssl.zerossl.apikey')}`,
    headers: {
      'Content-Type': 'application/x-www-form-urlencoded',
    },
    data,
  };

  const response = await axios(request)
    .catch((error) => {
      throw new Error(error);
    });
  return response;
}

module.exports = verifyDomain;
