import { requestApi } from './requestApi.js';
import { Certificate } from './Certificate.js';

/**
 * List ZeroSSL certificates
 *
 * @typedef {listCertificates}
 * @param {string} apiKey
 * @param {String[]} [statuses] - possible values: draft, pending_validation, issued, cancelled,
 * revoked, expired.
 * @param {string} [search]
 * @return {Promise<Certificate[]>}
 */

export async function listCertificates(apiKey, statuses = [], search = undefined) {
  let url = `https://api.zerossl.com/certificates?access_key=${apiKey}&limit=1000`;

  if (statuses.length > 0) {
    url += `&statuses=${statuses.join(',')}`;
  }

  if (search !== undefined) {
    url += `&search=${search}`;
  }

  const requestOptions = {
    method: 'GET',
    headers: {},
  };

  const data = await requestApi(url, requestOptions);

  return data.results.map((certificateData) => new Certificate(certificateData));
}
