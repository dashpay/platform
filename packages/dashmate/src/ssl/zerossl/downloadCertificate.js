/**
 * Download the certificate specified by id
 *
 * @typedef {downloadCertificate}
 * @param {string} id
 * @param {string} apiKey
 * @returns {Promise<string>}
 */
import { requestApi } from './requestApi.js';

export async function downloadCertificate(id, apiKey) {
  const url = `https://api.zerossl.com/certificates/${id}/download/return?access_key=${apiKey}`;

  const requestOptions = {
    method: 'GET',
    headers: { },
  };

  const data = await requestApi(url, requestOptions);

  return `${data['certificate.crt']}\n${data['ca_bundle.crt']}`;
}
