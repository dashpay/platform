const fs = require('fs');

/**
 * Save the ZeroSSL challenge
 *
 * @typedef {createCertificate}
 * @param {string} response
 * @param {string} homeDirPath
 * @param {Config} config
 * @return {Promise<string>}
 */
async function saveChallenge(
  response,
  homeDirPath,
  config,
) {
  const url = response.data.validation.other_methods[config.get('externalIp')].file_validation_url_http;
  const fileName = url.replace(`http://${config.get('externalIp')}/.well-known/pki-validation/`, '');
  let fileContent = '';
  for (let index = 0; index < 3; index++) {
    fileContent += response.data.validation.other_methods[config.get('externalIp')].file_validation_content[index];
    if (index < 2) {
      fileContent += '\n';
    }
  }

  const validationPath = `${homeDirPath}/ssl/.well-known/pki-validation/`;
  if (!fs.existsSync(validationPath)) {
    fs.mkdirSync(validationPath, { recursive: true });
  }

  const validationFile = validationPath + fileName;
  fs.writeFileSync(validationFile, fileContent, (err) => {
    if (err) { throw err; }
  });

  return { validationFile, fileName };
}

module.exports = saveChallenge;
