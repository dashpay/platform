const axios = require('axios');

/**
 * Create a ZeroSSL Certificate
 *
 * @typedef {createCertificate}
 * @param {string} csr
 * @param {Config} config
 * @return {Promise<string>}
 */
async function createCertificate(csr, config) {
  const data = new URLSearchParams({
    certificate_domains: config.get('externalIp'),
    certificate_validity_days: '90',
    certificate_csr: csr,
  }).toString();

  const request = {
    method: 'post',
    url: `https://api.zerossl.com/certificates?access_key=${config.get(
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

module.exports = createCertificate;
