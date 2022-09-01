const forge = require('node-forge');

/**
 * Generate a CSR
 *
 * @typedef {generateCsr}
 * @param {Object} keyPair
 * @param {string} [keyPair.publicKey]
 * @param {string} [keyPair.privateKey]
 * @param {string} externalIp
 * @return {Promise<string>}
 */
async function generateCsr(
  keyPair,
  externalIp,
) {
  const csr = forge.pki.createCertificationRequest();
  csr.publicKey = forge.pki.publicKeyFromPem(keyPair.publicKey);

  csr.setSubject([
    { name: 'commonName', value: externalIp },
    // { name: 'organizationName', value: 'Dash' },
  ]);

  csr.sign(forge.pki.privateKeyFromPem(keyPair.privateKey));

  return forge.pki.certificationRequestToPem(csr);
}

module.exports = generateCsr;
