import crypto from 'node:crypto';
import fs from 'node:fs';

/**
 * @param {string} chainFilePath
 * @param {string} privateFilePath
 * @return {boolean}
 */
export default function validateSslCertificateFiles(chainFilePath, privateFilePath) {
  const bundlePem = fs.readFileSync(chainFilePath, 'utf8');
  const privateKeyPem = fs.readFileSync(privateFilePath, 'utf8');

  // Step 2: Create a signature using the private key
  const data = 'This is a test message';
  const sign = crypto.createSign('SHA256');
  sign.update(data);
  sign.end();

  const signature = sign.sign(privateKeyPem, 'hex');

  // Verify the signature using the public key from the certificate
  const verify = crypto.createVerify('SHA256');
  verify.update(data);
  verify.end();

  // Extract the public key from the first certificate in the bundle
  const certificate = crypto.createPublicKey({
    key: bundlePem,
    format: 'pem',
  });

  return verify.verify(certificate, signature, 'hex');
}
