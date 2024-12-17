import fs from 'fs';
import path from 'path';
import validateZeroSslCertificateFactory, { ERRORS } from '../../../../src/ssl/zerossl/validateZeroSslCertificateFactory.js';

describe('validateZeroSslCertificateFactory', () => {
  let config;
  let expirationDays;
  let homeDir;
  let getCertificate;
  let validateZeroSslCertificate;

  beforeEach(function beforeEach() {
    config = {
      get: this.sinon.stub(),
      getName: this.sinon.stub(),
    };

    expirationDays = 30;

    homeDir = {
      joinPath: this.sinon.stub(),
    };

    getCertificate = this.sinon.stub();

    config.getName.returns('my-config');

    homeDir.joinPath.callsFake((...args) => path.join('/home/dir', ...args));

    config.get.withArgs('platform.gateway.ssl.providerConfigs.zerossl.apiKey').returns('test-api-key');
    config.get.withArgs('externalIp').returns('1.2.3.4');
    config.get.withArgs('platform.gateway.ssl.providerConfigs.zerossl.id').returns('certificate-id');

    this.sinon.stub(fs, 'existsSync').returns(true);

    validateZeroSslCertificate = validateZeroSslCertificateFactory(homeDir, getCertificate);
  });

  it('should return API_KEY_IS_NOT_SET error when apiKey is not set', async () => {
    config.get.withArgs('platform.gateway.ssl.providerConfigs.zerossl.apiKey').returns(null);

    const result = await validateZeroSslCertificate(config, expirationDays);

    expect(result.error).to.equal(ERRORS.API_KEY_IS_NOT_SET);
  });

  it('should return EXTERNAL_IP_IS_NOT_SET error when externalIp is not set', async () => {
    config.get.withArgs('externalIp').returns(null);

    const result = await validateZeroSslCertificate(config, expirationDays);

    expect(result.error).to.equal(ERRORS.EXTERNAL_IP_IS_NOT_SET);
  });

  it('should return CERTIFICATE_ID_IS_NOT_SET error when certificateId is not set', async () => {
    config.get.withArgs('platform.gateway.ssl.providerConfigs.zerossl.id').returns(null);

    const result = await validateZeroSslCertificate(config, expirationDays);

    expect(result.error).to.equal(ERRORS.CERTIFICATE_ID_IS_NOT_SET);
  });

  it('should return PRIVATE_KEY_IS_NOT_PRESENT error when private key file is not present', async function it() {
    const sslConfigDir = path.join('/home/dir', 'my-config', 'platform', 'gateway', 'ssl');
    const privateKeyFilePath = path.join(sslConfigDir, 'private.key');

    fs.existsSync.withArgs(privateKeyFilePath).returns(false);

    getCertificate.resolves({
      common_name: '1.2.3.4',
      status: 'issued',
      isExpiredInDays: this.sinon.stub().returns(false),
    });

    const result = await validateZeroSslCertificate(config, expirationDays);

    expect(result.error).to.equal(ERRORS.PRIVATE_KEY_IS_NOT_PRESENT);
  });

  it('should return EXTERNAL_IP_MISMATCH error when certificate common_name does not match externalIp', async function it() {
    const certificate = {
      common_name: '5.6.7.8',
      status: 'issued',
      isExpiredInDays: this.sinon.stub().returns(false),
    };

    getCertificate.resolves(certificate);

    const result = await validateZeroSslCertificate(config, expirationDays);

    expect(result.error).to.equal(ERRORS.EXTERNAL_IP_MISMATCH);
  });

  it('should return CERTIFICATE_IS_NOT_VALIDATED error when certificate status is pending_validation', async function it() {
    const certificate = {
      common_name: '1.2.3.4',
      status: 'pending_validation',
      isExpiredInDays: this.sinon.stub().returns(false),
    };

    getCertificate.resolves(certificate);

    const result = await validateZeroSslCertificate(config, expirationDays);

    expect(result.error).to.equal(ERRORS.CERTIFICATE_IS_NOT_VALIDATED);
    expect(result.data.certificate).to.equal(certificate);
  });

  it('should return CERTIFICATE_IS_NOT_VALIDATED error when certificate status is draft', async function it() {
    const certificate = {
      common_name: '1.2.3.4',
      status: 'draft',
      isExpiredInDays: this.sinon.stub().returns(false),
    };

    getCertificate.resolves(certificate);

    const result = await validateZeroSslCertificate(config, expirationDays);

    expect(result.error).to.equal(ERRORS.CERTIFICATE_IS_NOT_VALIDATED);
    expect(result.data.certificate).to.equal(certificate);
  });

  it('should return CSR_FILE_IS_NOT_PRESENT error when certificate is not issued and csr file is not present', async function it() {
    const certificate = {
      common_name: '1.2.3.4',
      status: 'revoked',
      isExpiredInDays: this.sinon.stub().returns(false),
    };

    getCertificate.resolves(certificate);

    const sslConfigDir = path.join('/home/dir', 'my-config', 'platform', 'gateway', 'ssl');
    const csrFilePath = path.join(sslConfigDir, 'csr.pem');

    fs.existsSync.withArgs(csrFilePath).returns(false);

    const result = await validateZeroSslCertificate(config, expirationDays);

    expect(result.error).to.equal(ERRORS.CSR_FILE_IS_NOT_PRESENT);
  });

  it('should return CERTIFICATE_IS_NOT_VALID error when certificate is not issued and csr file is present', async function it() {
    const certificate = {
      common_name: '1.2.3.4',
      status: 'revoked',
      isExpiredInDays: this.sinon.stub().returns(false),
    };

    getCertificate.resolves(certificate);

    this.sinon.stub(fs, 'readFileSync').returns('csr content');

    const result = await validateZeroSslCertificate(config, expirationDays);

    expect(result.error).to.equal(ERRORS.CERTIFICATE_IS_NOT_VALID);
    expect(result.data.csr).to.equal('csr content');
  });

  it('should return CERTIFICATE_EXPIRES_SOON error when certificate is expiring soon and csr file is present', async function it() {
    const certificate = {
      common_name: '1.2.3.4',
      status: 'issued',
      isExpiredInDays: this.sinon.stub().returns(true),
    };

    getCertificate.resolves(certificate);

    this.sinon.stub(fs, 'readFileSync').returns('csr content');

    const result = await validateZeroSslCertificate(config, expirationDays);

    expect(result.error).to.equal(ERRORS.CERTIFICATE_EXPIRES_SOON);
    expect(result.data.csr).to.equal('csr content');
  });

  it('should return data when certificate is valid and not expiring soon', async function it() {
    const certificate = {
      common_name: '1.2.3.4',
      status: 'issued',
      isExpiredInDays: this.sinon.stub().returns(false),
    };

    getCertificate.resolves(certificate);

    const result = await validateZeroSslCertificate(config, expirationDays);

    expect(result.error).to.be.undefined();
    expect(result.data).to.exist();
    expect(result.data.certificate).to.equal(certificate);
  });
});
