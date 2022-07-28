const axios = require('axios');
const fs = require('fs');
const wait = require('../../util/wait');

async function getCertificate(id, apikey) {
  const request = {
    method: 'get',
    url: `https://api.zerossl.com/certificates/${id}/download/return?access_key=${apikey}`,
    headers: { },
  };

  const response = await axios(request)
    .catch((error) => {
      throw new Error(error);
    });

  return response;
}

/**
 * Download the certificate specified by id
 *
 * @param {string} id
 * @param {string} homeDirPath
 * @param {Config} config
 */
async function downloadCertificate(id, homeDirPath, config) {
  const bundleFile = `${homeDirPath}/ssl/bundle.crt`;
  try {
    let response = '';
    do {
      await wait(2000);
      response = await getCertificate(id, config.get('platform.dapi.nginx.ssl.zerossl.apikey'));
    } while ('error' in response.data);

    fs.writeFileSync(
      bundleFile,
      `${response.data['certificate.crt']}\n${response.data['ca_bundle.crt']}`,
      (error) => {
        if (error) {
          throw error;
        }
      },
    );
  } catch (e) {
    throw new Error(e);
  }

  return bundleFile;
}

module.exports = downloadCertificate;
