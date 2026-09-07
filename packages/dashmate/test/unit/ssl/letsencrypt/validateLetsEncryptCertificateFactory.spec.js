import fs from 'node:fs';
import path from 'node:path';
import HomeDir from '../../../../src/config/HomeDir.js';
import createCertificateForTest from '../../../../src/test/createCertificateForTest.js';
import LegoCertificate from '../../../../src/ssl/letsencrypt/LegoCertificate.js';
import validateLetsEncryptCertificateFactory, { ERRORS } from '../../../../src/ssl/letsencrypt/validateLetsEncryptCertificateFactory.js';

const EXTERNAL_IP = '198.51.100.7';
const CONFIG_NAME = 'testnet';

describe('validateLetsEncryptCertificateFactory', () => {
  let homeDir;
  let legoDir;
  let sslDir;
  let config;
  let validateLetsEncryptCertificate;

  const issueCertificate = () => createCertificateForTest({ ip: EXTERNAL_IP, days: 60 });

  beforeEach(function beforeEach() {
    homeDir = HomeDir.createTemp();

    legoDir = homeDir.joinPath(CONFIG_NAME, 'platform', 'gateway', 'lego', 'certificates');
    sslDir = homeDir.joinPath(CONFIG_NAME, 'platform', 'gateway', 'ssl');

    fs.mkdirSync(legoDir, { recursive: true });
    fs.mkdirSync(sslDir, { recursive: true });

    config = {
      get: this.sinon.stub().callsFake((option) => ({
        'platform.gateway.ssl.providerConfigs.letsencrypt.email': 'operator@example.com',
        externalIp: EXTERNAL_IP,
      }[option])),
      getName: this.sinon.stub().returns(CONFIG_NAME),
    };

    validateLetsEncryptCertificate = validateLetsEncryptCertificateFactory(homeDir);
  });

  afterEach(() => homeDir.remove());

  // A certificate whose validity has not started is not servable, and the
  // gateway checks reject it. Judging it usable here would hand the rejected
  // certificate back to a repair that was meant to replace it.
  it('should not treat a certificate that is not valid yet as usable', () => {
    const notYetValid = new LegoCertificate({
      expires: new Date(Date.now() + 30 * 864e5),
      created: new Date(Date.now() + 864e5),
      commonName: EXTERNAL_IP,
      ipAddresses: [EXTERNAL_IP],
    });

    expect(notYetValid.isValid()).to.be.false();
  });

  it('should treat a certificate already inside its window as usable', () => {
    const current = new LegoCertificate({
      expires: new Date(Date.now() + 30 * 864e5),
      created: new Date(Date.now() - 864e5),
      commonName: EXTERNAL_IP,
      ipAddresses: [EXTERNAL_IP],
    });

    expect(current.isValid()).to.be.true();
  });

  // The gateway check requires the address in a subject alternative name,
  // because no standards-compliant client reads a common name for an IP. This
  // check deciding otherwise is what let a repair hand back the certificate the
  // gateway had already rejected.
  it('should not accept an address carried only in the common name', async () => {
    const { cert, key } = createCertificateForTest({ ip: EXTERNAL_IP, days: 60, withIpSan: false });

    fs.writeFileSync(path.join(legoDir, `${EXTERNAL_IP}.crt`), cert, 'utf8');
    fs.writeFileSync(path.join(legoDir, `${EXTERNAL_IP}.key`), key, 'utf8');
    fs.writeFileSync(path.join(sslDir, 'bundle.crt'), cert, 'utf8');
    fs.writeFileSync(path.join(sslDir, 'private.key'), key, 'utf8');

    const { error } = await validateLetsEncryptCertificate(config);

    expect(error).to.equal(ERRORS.CERTIFICATE_IP_MISMATCH);
  });

  it('should expose the not-installed error so callers can match on it', () => {
    expect(ERRORS.CERTIFICATE_NOT_INSTALLED).to.equal('CERTIFICATE_NOT_INSTALLED');
  });

  // A contact address is optional and nothing prompts for one, so no new node
  // has one. Its absence must not become the answer for every node - including
  // for the helper's own renewal scheduler - so it is not judged at all.
  it('should judge a certificate for a node that has no contact address', async function it() {
    config.get.callsFake((option) => ({
      'platform.gateway.ssl.providerConfigs.letsencrypt.email': null,
      externalIp: EXTERNAL_IP,
    }[option]));

    const { error } = await validateLetsEncryptCertificate(config);

    expect(error).to.equal(ERRORS.CERTIFICATE_NOT_FOUND);
  });

  it('should still report a missing external IP ahead of anything else', async function it() {
    config.get.callsFake((option) => ({
      'platform.gateway.ssl.providerConfigs.letsencrypt.email': null,
      externalIp: null,
    }[option]));

    const { error } = await validateLetsEncryptCertificate(config);

    expect(error).to.equal(ERRORS.EXTERNAL_IP_IS_NOT_SET);
  });

  it('should report no problem when the issued certificate is the one the gateway uses', async () => {
    const { cert, key } = issueCertificate();

    fs.writeFileSync(path.join(legoDir, `${EXTERNAL_IP}.crt`), cert);
    fs.writeFileSync(path.join(legoDir, `${EXTERNAL_IP}.key`), key);
    fs.writeFileSync(path.join(sslDir, 'bundle.crt'), cert);
    fs.writeFileSync(path.join(sslDir, 'private.key'), key);

    const result = await validateLetsEncryptCertificate(config);

    expect(result.error).to.be.undefined();
  });

  it('should report a renewed certificate that was never copied to the gateway', async () => {
    // Renewal writes a new certificate and then installs it for the gateway. When the second
    // step does not happen the node keeps serving the previous certificate until it expires,
    // and every check based on the renewed file alone still reports the node as healthy.
    const renewed = issueCertificate();
    const previous = issueCertificate();

    fs.writeFileSync(path.join(legoDir, `${EXTERNAL_IP}.crt`), renewed.cert);
    fs.writeFileSync(path.join(legoDir, `${EXTERNAL_IP}.key`), renewed.key);
    fs.writeFileSync(path.join(sslDir, 'bundle.crt'), previous.cert);
    fs.writeFileSync(path.join(sslDir, 'private.key'), previous.key);

    const result = await validateLetsEncryptCertificate(config);

    expect(result.error).to.equal('CERTIFICATE_NOT_INSTALLED');
  });

  it('should report a certificate that was issued but never installed at all', async () => {
    const { cert, key } = issueCertificate();

    fs.writeFileSync(path.join(legoDir, `${EXTERNAL_IP}.crt`), cert);
    fs.writeFileSync(path.join(legoDir, `${EXTERNAL_IP}.key`), key);

    const result = await validateLetsEncryptCertificate(config);

    expect(result.error).to.equal('CERTIFICATE_NOT_INSTALLED');
  });
});
