const os = require('os');
const path = require('path');
const fs = require('fs');
const generateKeyPair = require('../ssl/generateKeyPair');
const generateCsr = require('../ssl/zerossl/generateCsr');
const createCertificate = require('../ssl/selfSigned/createSelfSignedCertificate');

async function createSelfSignedCertificate(ip) {
  const keyPair = await generateKeyPair();
  const csr = await generateCsr(keyPair, ip);
  const certificate = await createCertificate(keyPair, csr);

  const tempDir = os.tmpdir();
  const certificatePath = path.join(tempDir, 'bundle.crt');
  const privKeyPath = path.join(tempDir, 'private.key');
  fs.writeFileSync(certificatePath, certificate, 'utf8');
  fs.writeFileSync(privKeyPath, keyPair.privateKey, 'utf8');
  return { certificatePath, privKeyPath };
}

module.exports = createSelfSignedCertificate;
