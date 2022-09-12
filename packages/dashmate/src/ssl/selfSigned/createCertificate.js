const forge = require('node-forge');

/**
 * @typedef {createSelfSignedCertificate}
 * @param {Object} keyPair
 * @param {string} [keyPair.publicKey]
 * @param {string} [keyPair.privateKey]
 * @param {string} csrPem
 * @return {Promise<{cert: string, key: string}>}
 */
async function createCertificate(keyPair, csrPem) {
  const cert = forge.pki.createCertificate();
  const csr = forge.pki.certificationRequestFromPem(csrPem);

  cert.publicKey = csr.publicKey;
  cert.serialNumber = '01';
  cert.validity.notBefore = new Date();
  cert.validity.notAfter = new Date();
  cert.validity.notAfter.setFullYear(cert.validity.notBefore.getFullYear() + 1);

  cert.setSubject(csr.subject.attributes);
  cert.setIssuer(csr.subject.attributes);

  cert.sign(forge.pki.privateKeyFromPem(keyPair.privateKey));

  return forge.pki.certificateToPem(cert);
}

module.exports = createCertificate;
