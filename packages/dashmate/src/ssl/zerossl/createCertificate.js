const fetch = require('node-fetch');
const qs = require('qs');
const errorDescriptions = require('./errors/errorDescriptions');
const Certificate = require('./Certificate');

/**
 * Create a ZeroSSL Certificate
 *
 * @typedef {createCertificate}
 * @param {string} csr
 * @param {string} externalIp
 * @param {string} apiKey
 * @return {Promise<Certificate>}
 */
async function createCertificate(
  csr,
  externalIp,
  apiKey,
) {
  const body = qs.stringify({
    certificate_domains: externalIp,
    certificate_validity_days: '90',
    certificate_csr: csr,
  });

  const url = `https://api.zerossl.com/certificates?access_key=${apiKey}`;

  const requestOptions = {
    method: 'POST',
    body,
    headers: {
      'Content-Type': 'application/x-www-form-urlencoded',
    },
  };

  const response = await fetch(url, requestOptions);

  const data = await response.json();

  if (data.error) {
    const errorMessage = errorDescriptions[data.error.code];

    throw new Error(errorMessage || JSON.stringify(data.error));
  }

  return new Certificate(data);
}

module.exports = createCertificate;
