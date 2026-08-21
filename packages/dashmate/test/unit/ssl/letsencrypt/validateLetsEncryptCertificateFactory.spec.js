import fs from 'node:fs';
import path from 'node:path';
import HomeDir from '../../../../src/config/HomeDir.js';
import createCertificateForTest from '../../../../src/test/createCertificateForTest.js';
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

  it('should expose the not-installed error so callers can match on it', () => {
    expect(ERRORS.CERTIFICATE_NOT_INSTALLED).to.equal('CERTIFICATE_NOT_INSTALLED');
  });

  // The email check used to fire before every other one, so a node without a
  // contact address reported EMAIL_IS_NOT_SET whatever else was wrong with its
  // certificate. Nothing prompts for an address any more, so no new node has
  // one and this would have become the answer for all of them - including for
  // the helper's own renewal scheduler.
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
