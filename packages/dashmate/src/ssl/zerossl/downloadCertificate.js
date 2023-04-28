const fetch = require('node-fetch');
const wait = require('../../util/wait');
const errorDescriptions = require('./errors/errorDescriptions');

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

  let data;
  let success = false;

  do {
    try {
      await wait(2000);
      const response = await fetch(url, requestOptions);
      data = await response.json();

      ({ success } = data);
    } catch (e) {
      // do nothing
    }
  } while (success === false && Date.now() - startedAt < maxTime);

  if (!data) {
    throw new Error('Can\'t download certificate: max time limit has been reached');
  }

  if (data.error) {
    const errorMessage = errorDescriptions[data.error.code];

    throw new Error(errorMessage || JSON.stringify(data.error));
  }

  return `${data['certificate.crt']}\n${data['ca_bundle.crt']}`;
}

module.exports = downloadCertificate;
