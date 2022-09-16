const axios = require('axios');
const wait = require('../../util/wait');

/**
 * Download the certificate specified by id
 *
 * @typedef {downloadCertificate}
 * @param {string} id
 * @param {string} apiKey
 * @returns {string}
 */
async function downloadCertificate(id, apiKey) {
  const maxTime = 10 * 60 * 1000; // 10 minutes
  const startedAt = Date.now();

  const request = {
    method: 'get',
    url: `https://api.zerossl.com/certificates/${id}/download/return?access_key=${apiKey}`,
    headers: { },
  };

  let response;

  do {
    await wait(2000);
    response = await axios(request);
  } while (response.data.success === false && Date.now() - startedAt < maxTime);

  if (!response) {
    throw new Error('Can\'t download certificate: max time limit has been reached');
  }

  return `${response.data['certificate.crt']}\n${response.data['ca_bundle.crt']}`;
}

module.exports = downloadCertificate;
