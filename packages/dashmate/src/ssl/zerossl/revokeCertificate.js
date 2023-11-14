import requestApi from './requestApi.js';

/**
 * Create a ZeroSSL Certificate
 *
 * @typedef {revokeCertificate}
 * @param {string} apiKey
 * @param {string} id
 * @return {Promise<Certificate>}
 */
export default async function revokeCertificate(
  apiKey,
  id,
) {
  const url = `https://api.zerossl.com/certificates/${id}/revoke?access_key=${apiKey}`;

  const requestOptions = {
    method: 'POST',
    headers: {
      'Content-Type': 'application/x-www-form-urlencoded',
    },
  };

  return requestApi(url, requestOptions);
}
