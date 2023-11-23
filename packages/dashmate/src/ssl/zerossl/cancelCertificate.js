import requestApi from './requestApi';

/**
 * Get ZeroSSL certificate
 *
 * @typedef {getCertificate}
 * @param {string} apiKey
 * @param {string} id
 * @return {Promise<Certificate>}
 */
export default async function cancelCertificate(apiKey, id) {
  const url = `https://api.zerossl.com/certificates/${id}/cancel?access_key=${apiKey}`;

  const requestOptions = {
    method: 'POST',
    headers: {
      'Content-Type': 'application/x-www-form-urlencoded',
    },
  };

  return requestApi(url, requestOptions);
}
