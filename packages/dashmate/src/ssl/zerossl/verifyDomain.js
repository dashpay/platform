const fetch = require('node-fetch');

const qs = require('qs');

/**
 * Verify the domain/ip specified by certificate id
 *
 * @typedef {verifyDomain}
 * @param {string} id
 * @param {string} apiKey
 * @return {Promise<Object>}
 */
async function verifyDomain(id, apiKey) {
  const body = qs.stringify({
    validation_method: 'HTTP_CSR_HASH',
  });

  const url = `https://api.zerossl.com/certificates/${id}/challenges?access_key=${apiKey}`;

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
    throw new Error(data.error.type);
  }

  return data;
}

module.exports = verifyDomain;
