import crypto from 'crypto';
import forge from 'node-forge';
import generateCsr from '../../../../src/ssl/zerossl/generateCsr.js';
import loadCertificateRequest from '../../../../src/ssl/zerossl/loadCertificateRequest.js';

/**
 * @param {crypto.KeyObject|string} key
 * @return {Buffer} its public key as SubjectPublicKeyInfo
 */
function toSpki(key) {
  return crypto.createPublicKey(key).export({ type: 'spki', format: 'der' });
}

/**
 * @param {string} pem - a certificate request
 * @return {{ oid: string, type: number, value: string }} its subject's one
 *  attribute
 */
function readSubjectAttribute(pem) {
  const [block] = forge.pem.decode(pem);
  const request = forge.asn1.fromDer(block.body);

  // CertificationRequestInfo, its subject, the RDN's SET, the attribute
  const [type, value] = request.value[0].value[1].value[0].value[0].value;

  return { oid: forge.asn1.derToOid(type.value), type: value.type, value: value.value };
}

/**
 * @param {string} type
 * @param {Object} [options]
 * @return {{ publicKey: string, privateKey: string }} PEM, as the task passes them
 */
function generatePemKeyPair(type, options = {}) {
  return crypto.generateKeyPairSync(type, {
    ...options,
    publicKeyEncoding: { type: 'spki', format: 'pem' },
    privateKeyEncoding: { type: 'pkcs8', format: 'pem' },
  });
}

describe('generateCsr', () => {
  // What renewal sends to ZeroSSL has to be the request obtain itself accepts
  // as usable for the key on disk.
  [
    ['an RSA key', () => generatePemKeyPair('rsa', { modulusLength: 2048 })],
    ['an EC P-256 key', () => generatePemKeyPair('ec', { namedCurve: 'prime256v1' })],
    ['an EC P-384 key', () => generatePemKeyPair('ec', { namedCurve: 'secp384r1' })],
  ].forEach(([description, generate]) => {
    it(`should write a request for ${description} that obtain can renew with`, async () => {
      const keyPair = generate();

      const pem = await generateCsr(keyPair, '1.2.3.4');

      const request = loadCertificateRequest(pem);
      expect(request).to.not.be.null();
      expect(request.publicKey.export({ type: 'spki', format: 'der' }))
        .to.deep.equal(toSpki(keyPair.privateKey));
      // The common name as a PrintableString, as node-forge writes it
      expect(readSubjectAttribute(pem)).to.deep.equal({
        oid: '2.5.4.3',
        type: forge.asn1.Type.PRINTABLESTRING,
        value: '1.2.3.4',
      });
    });
  });

  // ZeroSSL issues certificates for RSA and EC keys only.
  it('should refuse a key of another type, naming it', async () => {
    const keyPair = generatePemKeyPair('ed25519');

    await expect(generateCsr(keyPair, '1.2.3.4'))
      .to.be.rejectedWith('ed25519 key: dashmate writes them for RSA and EC keys');
  });
});
