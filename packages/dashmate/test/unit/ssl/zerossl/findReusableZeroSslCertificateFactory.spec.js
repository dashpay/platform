import fs from 'fs';
import path from 'path';
import HomeDir from '../../../../src/config/HomeDir.js';
import getBaseConfigFactory from '../../../../configs/defaults/getBaseConfigFactory.js';
import validateZeroSslCertificateFactory from '../../../../src/ssl/zerossl/validateZeroSslCertificateFactory.js';
import findReusableZeroSslCertificateFactory from '../../../../src/ssl/zerossl/findReusableZeroSslCertificateFactory.js';
import Certificate from '../../../../src/ssl/zerossl/Certificate.js';
import ProviderUnreachableError from '../../../../src/ssl/errors/ProviderUnreachableError.js';
import { issueCertificate } from '../../../../src/test/certificateFixtures.js';

const DAY_MS = 24 * 60 * 60 * 1000;

/**
 * ZeroSSL's own date format
 *
 * @param {Date} date
 * @return {string}
 */
function toZeroSslDate(date) {
  const pad = (number) => String(number).padStart(2, '0');

  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`
    + ` ${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`;
}

describe('findReusableZeroSslCertificate', () => {
  let pair;
  let homeDir;
  let config;
  let sslDir;
  let getCertificate;
  let findReusableZeroSslCertificate;

  /**
   * What ZeroSSL answers about the saved certificate
   *
   * @param {string} status
   * @param {number} [expiresInDays]
   * @return {Certificate}
   */
  function zeroSslCertificate(status, expiresInDays = 60) {
    return new Certificate({
      id: 'c'.repeat(32),
      common_name: '1.2.3.4',
      status,
      created: toZeroSslDate(new Date(Date.now() - DAY_MS)),
      expires: status === 'issued' || status === 'expired'
        ? toZeroSslDate(new Date(Date.now() + expiresInDays * DAY_MS))
        : null,
    });
  }

  before(function before() {
    this.timeout(30000);
    pair = issueCertificate({ ip: '1.2.3.4' });
  });

  beforeEach(function beforeEach() {
    homeDir = HomeDir.createTemp();
    config = getBaseConfigFactory(homeDir)();
    config.set('externalIp', '1.2.3.4');
    config.set('platform.gateway.ssl.provider', 'zerossl');
    config.set('platform.gateway.ssl.providerConfigs.zerossl.apiKey', 'k'.repeat(32));
    config.set('platform.gateway.ssl.providerConfigs.zerossl.id', 'c'.repeat(32));

    // A run that stopped while writing the bundle: the key and the request
    // are whole, the bundle is cut short.
    sslDir = homeDir.joinPath(config.getName(), 'platform', 'gateway', 'ssl');
    fs.mkdirSync(sslDir, { recursive: true });
    fs.writeFileSync(path.join(sslDir, 'private.key'), pair.keyPem);
    fs.writeFileSync(path.join(sslDir, 'csr.pem'), 'certificate-request');
    fs.writeFileSync(path.join(sslDir, 'bundle.crt'), pair.pem.slice(0, pair.pem.length / 2));

    getCertificate = this.sinon.stub().resolves(zeroSslCertificate('issued'));

    findReusableZeroSslCertificate = findReusableZeroSslCertificateFactory(
      validateZeroSslCertificateFactory(homeDir, getCertificate),
    );
  });

  afterEach(() => {
    homeDir.remove();
  });

  it('should reuse a saved certificate ZeroSSL issued, whose key is on this node', async () => {
    expect(await findReusableZeroSslCertificate(config)).to.deep.equal({
      isReusable: true,
      status: 'issued',
      reason: null,
    });
  });

  it('should reuse a saved certificate ZeroSSL still validates', async () => {
    getCertificate.resolves(zeroSslCertificate('pending_validation'));

    expect(await findReusableZeroSslCertificate(config)).to.include({
      isReusable: true,
      status: 'pending_validation',
    });
  });

  it('should not reuse anything without a saved certificate ID', async () => {
    config.set('platform.gateway.ssl.providerConfigs.zerossl.id', null);

    expect(await findReusableZeroSslCertificate(config)).to.include({ isReusable: false });
    expect(getCertificate).to.not.have.been.called();
  });

  ['cancelled', 'expired', 'revoked'].forEach((status) => {
    it(`should not reuse a certificate ZeroSSL reports ${status}`, async () => {
      getCertificate.resolves(zeroSslCertificate(status));

      const reusable = await findReusableZeroSslCertificate(config);

      expect(reusable.isReusable).to.be.false();
      expect(reusable.reason).to.include(status);
    });
  });

  // The key the certificate was requested with is the only one it serves with.
  it('should not reuse an issued certificate whose key is not on this node', async () => {
    fs.rmSync(path.join(sslDir, 'private.key'));

    const reusable = await findReusableZeroSslCertificate(config);

    expect(reusable.isReusable).to.be.false();
    expect(reusable.reason).to.include('private key');
  });

  it('should not reuse a certificate that expires within the days an obtain renews it', async () => {
    getCertificate.resolves(zeroSslCertificate('issued', Certificate.EXPIRATION_LIMIT_DAYS - 1));

    expect(await findReusableZeroSslCertificate(config)).to.include({ isReusable: false });
  });

  it('should not know when ZeroSSL cannot be reached', async () => {
    getCertificate.rejects(new ProviderUnreachableError('fetch failed'));

    expect(await findReusableZeroSslCertificate(config)).to.deep.equal({
      isReusable: null,
      status: null,
      reason: 'fetch failed',
    });
  });

  it('should not know when ZeroSSL answers with an error', async () => {
    getCertificate.rejects(Object.assign(new Error('Internal error'), { code: 2900 }));

    expect(await findReusableZeroSslCertificate(config)).to.include({
      isReusable: null,
      reason: 'Internal error',
    });
  });

  // A refusal waits for this answer, so it is bounded.
  it('should ask ZeroSSL with a time limit', async () => {
    await findReusableZeroSslCertificate(config);

    expect(getCertificate.firstCall.args[2].signal).to.be.an.instanceOf(AbortSignal);
  });
});
