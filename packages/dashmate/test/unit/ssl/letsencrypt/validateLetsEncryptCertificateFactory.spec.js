import { execFileSync } from 'node:child_process';
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import HomeDir from '../../../../src/config/HomeDir.js';
import validateLetsEncryptCertificateFactory, { ERRORS } from '../../../../src/ssl/letsencrypt/validateLetsEncryptCertificateFactory.js';

const EXTERNAL_IP = '198.51.100.7';
const CONFIG_NAME = 'testnet';

describe('validateLetsEncryptCertificateFactory', () => {
  let homeDir;
  let legoDir;
  let sslDir;
  let config;
  let validateLetsEncryptCertificate;

  /**
   * @return {{cert: string, key: string}}
   */
  function issueCertificate() {
    const { privateKey } = crypto.generateKeyPairSync('rsa', { modulusLength: 2048 });
    const key = privateKey.export({ type: 'pkcs8', format: 'pem' }).toString();

    const dir = fs.mkdtempSync(path.join(homeDir.getPath(), 'issue-'));
    const keyPath = path.join(dir, 'key.pem');
    const certPath = path.join(dir, 'cert.pem');

    fs.writeFileSync(keyPath, key);

    execFileSync('openssl', [
      'req', '-x509', '-new', '-key', keyPath, '-out', certPath,
      '-subj', `/CN=${EXTERNAL_IP}`,
      '-addext', `subjectAltName=IP:${EXTERNAL_IP}`,
      '-addext', 'basicConstraints=CA:FALSE',
      '-days', '60',
    ], { stdio: 'ignore' });

    return { cert: fs.readFileSync(certPath, 'utf8'), key };
  }

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
