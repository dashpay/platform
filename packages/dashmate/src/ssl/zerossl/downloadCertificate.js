const fetch = require('node-fetch');
const wait = require('../../util/wait');

/**
 * Download the certificate specified by id
 *
 * @typedef {downloadCertificate}
 * @param {string} id
 * @param {string} apiKey
 * @returns {Promise<string>}
 */
async function downloadCertificate(id, apiKey) {
  const maxTime = 10 * 60 * 1000; // 10 minutes
  const startedAt = Date.now();

  const url = `https://api.zerossl.com/certificates/${id}/download/return?access_key=${apiKey}`;

  const requestOptions = {
    method: 'GET',
    headers: { },
  };

  let response;

  do {
    await wait(2000);
    response = await fetch(url, requestOptions);
  } while (!response.ok && Date.now() - startedAt < maxTime);

  if (!response.ok) {
    throw new Error('Can\'t download certificate: max time limit has been reached');
  }

  const data = await response.json();

  return `${data['certificate.crt']}\n${data['ca_bundle.crt']}`;
}

module.exports = downloadCertificate;
