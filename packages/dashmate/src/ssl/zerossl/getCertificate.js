import Certificate from './Certificate.js';
import requestApi from './requestApi.js';

/**
 * Get ZeroSSL certificate
 *
 * @typedef {getCertificate}
 * @param {string} apiKey
 * @param {string} id
 * @param {Object} [options]
 * @param {AbortSignal} [options.signal] - ends the request
 * @return {Promise<Certificate>}
 */
export default async function getCertificate(apiKey, id, { signal } = {}) {
  const url = `https://api.zerossl.com/certificates/${id}`;

  const requestOptions = {
    method: 'GET',
    headers: { },
    ...(signal ? { signal } : {}),
  };

  const data = await requestApi(apiKey, url, requestOptions);

  return new Certificate(data);
}
