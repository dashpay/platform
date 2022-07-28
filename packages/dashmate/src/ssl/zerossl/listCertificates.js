const axios = require('axios');

/**
 * List ZeroSSL certificates
 *
 * @param {string} apiKey
 */
async function listCertificates(apiKey) {
  const config = {
    method: 'get',
    url: `https://api.zerossl.com/certificates?access_key=${apiKey}`,
    headers: { },
  };

  const response = await axios(config)
    .catch((error) => {
      throw new Error(error);
    });
  return response;
}

module.exports = listCertificates;
