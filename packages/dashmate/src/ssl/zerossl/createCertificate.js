const axios = require('axios');
const qs = require('qs');

/**
 * Create a ZeroSSL Certificate
 *
 * @typedef {createZerosslCertificate}
 * @param {string} csr
 * @param {string} externalIp
 * @param {string} apiKey
 * @return {Promise<string>}
 */
async function createCertificate(
  csr,
  externalIp,
  apiKey,
) {
  const data = qs.stringify({
    certificate_domains: externalIp,
    certificate_validity_days: '90',
    certificate_csr: csr,
  });

  const request = {
    method: 'post',
    url: `https://api.zerossl.com/certificates?access_key=${apiKey}`,
    headers: {
      'Content-Type': 'application/x-www-form-urlencoded',
    },
    data,
  };

  const response = await axios(request);

  if (response.data.error) {
    throw new Error(JSON.stringify(response.data.error));
  }

  return response.data;
}

module.exports = createCertificate;
