const axios = require('axios');
const wait = require('../../util/wait');

async function queryTempServer(serverURL) {
  const request = {
    method: 'get',
    url: serverURL,
    headers: { },
  };

  const response = await axios(request)
    .catch((error) => {
      throw new Error(error);
    });
  return response;
}

/**
 * Setup temp server for ZeroSSL challenge
 *
 * @typedef {verifyTempServer}
 * @param {string} challengeFile
 * @param {string} externalIp
 * @return {Promise<string>}
 */
async function verifyTempServer(
  challengeFile,
  externalIp,
) {
  let response;
  do {
    await wait(1000);
    response = await queryTempServer(`http://${externalIp}/.well-known/pki-validation/${challengeFile}`);
  } while (!response.data);

  return response.data;
}

module.exports = verifyTempServer;
