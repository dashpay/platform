import qs from 'qs';
import Certificate from './Certificate.js';
import requestApi from './requestApi.js';

/**
 * Create a ZeroSSL Certificate
 *
 * @typedef {createZeroSSLCertificate}
 * @param {string} csr
 * @param {string} externalIp
 * @param {string} apiKey
 * @return {Promise<Certificate>}
 */
export default async function createZeroSSLCertificate(
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

  const data = await requestApi(url, requestOptions);

  return new Certificate(data);
}
