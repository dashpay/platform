const fs = require('fs');
const execa = require('execa');

/**
 * Generate a CSR
 *
 * @typedef {createCertificate}
 * @param {string} externalIp
 * @param {string} homeDirPath
 * @return {undefined}
 */
async function generateCsr(
  externalIp,
  homeDirPath,
) {
  if (externalIp === null) {
    throw new Error('External IP must be defined');
  }

  fs.mkdirSync(`${homeDirPath}/ssl`, { recursive: true });

  // Should we use this instead? https://www.npmjs.com/package/pem
  try {
    await execa('openssl',
      [
        'req', '-new',
        '-newkey', 'rsa:2048', '-nodes', '-keyout', `${homeDirPath}/private.key`, // private.key
        '-out', `${homeDirPath}/ssl/domain.csr`, '-subj', `/CN=${externalIp}`, // domain.csr
      ]);
  } catch (error) {
    throw new Error(error);
  }

  return undefined;
}

module.exports = generateCsr;
