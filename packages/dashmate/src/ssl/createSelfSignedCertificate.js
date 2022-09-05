const forge = require('node-forge');

/**
 * @typedef {createSelfSignedCertificate}
 * @param {Object} keyPair
 * @param {string} [keyPair.publicKey]
 * @param {string} [keyPair.privateKey]
 * @param {string} csrPem
 * @return {Promise<{cert: string, key: string}>}
 */
async function createSelfSignedCertificate(keyPair, csrPem) {
  const cert = forge.pki.createCertificate();
  const csr = forge.pki.certificationRequestFromPem(csrPem);

  cert.publicKey = csr.publicKey;
  cert.serialNumber = '01';
  cert.validity.notBefore = new Date();
  cert.validity.notAfter = new Date();
  cert.validity.notAfter.setFullYear(cert.validity.notBefore.getFullYear() + 1);

  cert.setSubject(csr.subject.attributes);
  cert.setIssuer(csr.subject.attributes);

  const extensionRequest = csr.getAttribute({ name: 'extensionRequest' });

  if (extensionRequest) {
    const { extensions } = extensionRequest;
    extensions.push.apply(extensions, [{
      name: 'basicConstraints',
      cA: true,
    }, {
      name: 'keyUsage',
      keyCertSign: true,
      digitalSignature: true,
      nonRepudiation: true,
      keyEncipherment: true,
      dataEncipherment: true,
    }]);
    cert.setExtensions(extensions);
  }

  cert.sign(forge.pki.privateKeyFromPem(keyPair.privateKey));

  return forge.pki.certificateToPem(cert);
}

module.exports = createSelfSignedCertificate;
