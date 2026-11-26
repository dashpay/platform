import crypto from 'crypto';
import fs from 'fs';
import path from 'path';
import { execFileSync } from 'child_process';
import forge from 'node-forge';
import validateZeroSslCertificateFactory, { ERRORS } from '../../../../src/ssl/zerossl/validateZeroSslCertificateFactory.js';
import Certificate from '../../../../src/ssl/zerossl/Certificate.js';
import HomeDir from '../../../../src/config/HomeDir.js';
import generateCsr from '../../../../src/ssl/zerossl/generateCsr.js';
import { createCertificateRequest, issueCertificate } from '../../../../src/test/certificateFixtures.js';

describe('validateZeroSslCertificateFactory', () => {
  let config;
  let expirationDays;
  let homeDir;
  let getCertificate;
  let validateZeroSslCertificate;
  let csrPem;

  before(async () => {
    const { publicKey, privateKey } = crypto.generateKeyPairSync('rsa', {
      modulusLength: 2048,
      publicKeyEncoding: { type: 'spki', format: 'pem' },
      privateKeyEncoding: { type: 'pkcs1', format: 'pem' },
    });

    csrPem = await generateCsr({ publicKey, privateKey }, '1.2.3.4');
  });

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

    this.sinon.stub(fs, 'statSync').returns({ isFile: () => true });

    validateZeroSslCertificate = validateZeroSslCertificateFactory(homeDir, getCertificate);
  });

  let requestHomeDir;

  /**
   * Put the certificate request where the validator reads it
   */
  function writeCertificateRequest() {
    requestHomeDir = HomeDir.createTemp();
    homeDir.joinPath.callsFake((...args) => requestHomeDir.joinPath(...args));

    const sslConfigDir = requestHomeDir.joinPath('my-config', 'platform', 'gateway', 'ssl');
    fs.mkdirSync(sslConfigDir, { recursive: true });
    fs.writeFileSync(path.join(sslConfigDir, 'csr.pem'), csrPem);
  }

  afterEach(() => {
    requestHomeDir?.remove();
    requestHomeDir = undefined;
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

    fs.statSync.withArgs(privateKeyFilePath).returns(undefined);

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

    fs.statSync.withArgs(csrFilePath).returns(undefined);

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

    writeCertificateRequest();

    const result = await validateZeroSslCertificate(config, expirationDays);

    expect(result.error).to.equal(ERRORS.CERTIFICATE_IS_NOT_VALID);
    expect(result.data.csr).to.equal(csrPem);
  });

  it('should return CERTIFICATE_EXPIRES_SOON error when certificate is expiring soon and csr file is present', async function it() {
    const certificate = {
      common_name: '1.2.3.4',
      status: 'issued',
      isExpiredInDays: this.sinon.stub().returns(true),
    };

    getCertificate.resolves(certificate);

    writeCertificateRequest();

    const result = await validateZeroSslCertificate(config, expirationDays);

    expect(result.error).to.equal(ERRORS.CERTIFICATE_EXPIRES_SOON);
    expect(result.data.csr).to.equal(csrPem);
  });

  // A certificate the provider has not issued yet carries no expiry date. Reporting it as
  // expiring would put that missing date in front of the operator; it is unapproved, not expired.
  it('should report a certificate without an expiry date as not validated', async function it() {
    const certificate = new Certificate({
      id: 'certificate-id',
      common_name: '1.2.3.4',
      status: 'pending_validation',
      created: '2026-08-18 09:04:19',
      expires: null,
    });

    getCertificate.resolves(certificate);

    const result = await validateZeroSslCertificate(config, expirationDays);

    expect(result.error).to.equal(ERRORS.CERTIFICATE_IS_NOT_VALIDATED);
    expect(result.error).to.not.equal(ERRORS.CERTIFICATE_EXPIRES_SOON);
    expect(result.data.isExpiresSoon).to.be.false();
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

  // The files the certificate was installed as, on a real disk: an obtain
  // downloads a bundle again unless it counts as there.
  describe('the certificate files on disk', () => {
    let pair;
    let realHomeDir;
    let sslConfigDir;

    before(function before() {
      this.timeout(30000);
      pair = issueCertificate({ ip: '1.2.3.4' });
    });

    beforeEach(function beforeEach() {
      fs.statSync.restore();

      realHomeDir = HomeDir.createTemp();
      sslConfigDir = realHomeDir.joinPath('my-config', 'platform', 'gateway', 'ssl');
      fs.mkdirSync(sslConfigDir, { recursive: true });
      fs.writeFileSync(path.join(sslConfigDir, 'csr.pem'), 'certificate-request');

      getCertificate.resolves({
        common_name: '1.2.3.4',
        status: 'issued',
        isExpiredInDays: this.sinon.stub().returns(false),
      });

      validateZeroSslCertificate = validateZeroSslCertificateFactory(realHomeDir, getCertificate);
    });

    afterEach(() => {
      realHomeDir.remove();
    });

    it('should count the bundle of the saved certificate as downloaded', async function it() {
      fs.writeFileSync(path.join(sslConfigDir, 'bundle.crt'), pair.pem);
      fs.writeFileSync(path.join(sslConfigDir, 'private.key'), pair.keyPem);
      getCertificate.resolves({
        common_name: '1.2.3.4',
        status: 'issued',
        fingerprint_sha1: new crypto.X509Certificate(pair.pem).fingerprint.replaceAll(':', '').toLowerCase(),
        isExpiredInDays: this.sinon.stub().returns(false),
      });

      const result = await validateZeroSslCertificate(config, expirationDays);

      expect(result.error).to.be.undefined();
      expect(result.data.isBundleFilePresent).to.be.true();
    });

    // Renewal saves the new certificate's ID before its bundle is written,
    // and the bundle of the certificate it replaces serves until it expires.
    it('should not count the bundle of another certificate for the key as downloaded', async function it() {
      const renewed = issueCertificate({ ip: '1.2.3.4', keys: pair.keys, days: 60 });
      fs.writeFileSync(path.join(sslConfigDir, 'bundle.crt'), pair.pem);
      fs.writeFileSync(path.join(sslConfigDir, 'private.key'), pair.keyPem);
      getCertificate.resolves({
        common_name: '1.2.3.4',
        status: 'issued',
        fingerprint_sha1: new crypto.X509Certificate(renewed.pem).fingerprint,
        isExpiredInDays: this.sinon.stub().returns(false),
      });

      const result = await validateZeroSslCertificate(config, expirationDays);

      expect(result.error).to.be.undefined();
      expect(result.data.isBundleFilePresent).to.be.false();
    });

    // Without ZeroSSL's fingerprint nothing ties the bundle to the saved ID,
    // and downloading it by that ID costs nothing.
    it('should not count a bundle as downloaded when ZeroSSL gives no fingerprint', async () => {
      fs.writeFileSync(path.join(sslConfigDir, 'bundle.crt'), pair.pem);
      fs.writeFileSync(path.join(sslConfigDir, 'private.key'), pair.keyPem);

      const result = await validateZeroSslCertificate(config, expirationDays);

      expect(result.error).to.be.undefined();
      expect(result.data.isBundleFilePresent).to.be.false();
    });

    // A full disk cuts the bundle short after the key was written.
    it('should not count a truncated bundle as downloaded', async () => {
      fs.writeFileSync(path.join(sslConfigDir, 'bundle.crt'), pair.pem.slice(0, pair.pem.length / 2));
      fs.writeFileSync(path.join(sslConfigDir, 'private.key'), pair.keyPem);

      const result = await validateZeroSslCertificate(config, expirationDays);

      expect(result.error).to.be.undefined();
      expect(result.data.isBundleFilePresent).to.be.false();
    });

    // The gateway runs as another user and may read what this process cannot.
    // Reading is not writing: a bundle that cannot be judged is left as it is.
    it('should leave a bundle this process cannot read as it is', async () => {
      fs.writeFileSync(path.join(sslConfigDir, 'bundle.crt'), pair.pem, { mode: 0o200 });
      fs.writeFileSync(path.join(sslConfigDir, 'private.key'), pair.keyPem);

      const result = await validateZeroSslCertificate(config, expirationDays);

      expect(result.data.isBundleFilePresent).to.be.true();
    });

    // Opening a named pipe without a writer never returns.
    it('should not open a named pipe where the bundle goes', async () => {
      execFileSync('mkfifo', [path.join(sslConfigDir, 'bundle.crt')]);
      fs.writeFileSync(path.join(sslConfigDir, 'private.key'), pair.keyPem);

      const result = await validateZeroSslCertificate(config, expirationDays);

      expect(result.data.isBundleFilePresent).to.be.true();
    });

    // A full disk can cut the key short too. The certificate serves with
    // nothing else.
    it('should not count a key cut short as the key', async () => {
      fs.writeFileSync(path.join(sslConfigDir, 'private.key'), pair.keyPem.slice(0, pair.keyPem.length / 2));

      const result = await validateZeroSslCertificate(config, expirationDays);

      expect(result.error).to.equal(ERRORS.PRIVATE_KEY_IS_NOT_PRESENT);
    });

    // Compose creates a missing bind-mount source as a directory. The key the
    // certificate was requested with is gone.
    it('should not count a directory where the key goes as the key', async () => {
      fs.mkdirSync(path.join(sslConfigDir, 'private.key'));

      const result = await validateZeroSslCertificate(config, expirationDays);

      expect(result.error).to.equal(ERRORS.PRIVATE_KEY_IS_NOT_PRESENT);
    });

    // Renewal sends csr.pem to ZeroSSL as it is, so only a request that
    // parses and carries the key on disk can renew the certificate.
    describe('the certificate request', () => {
      let request;
      let csrFilePath;

      before(async () => {
        request = await generateCsr({
          publicKey: forge.pki.publicKeyToPem(pair.keys.publicKey),
          privateKey: pair.keyPem,
        }, '1.2.3.4');
      });

      beforeEach(() => {
        csrFilePath = path.join(sslConfigDir, 'csr.pem');
        fs.writeFileSync(path.join(sslConfigDir, 'private.key'), pair.keyPem);
      });

      it('should count a request for the key on disk as the request', async () => {
        fs.writeFileSync(csrFilePath, request);

        const result = await validateZeroSslCertificate(config, expirationDays);

        expect(result.data.isCsrFilePresent).to.be.true();
      });

      // A full disk cuts the request short like the files after it.
      it('should not count a request cut short as the request', async () => {
        fs.writeFileSync(csrFilePath, request.slice(0, request.length / 2));

        const result = await validateZeroSslCertificate(config, expirationDays);

        expect(result.error).to.be.undefined();
        expect(result.data.isCsrFilePresent).to.be.false();
      });

      it('should not count a request for another key as the request', async () => {
        const { publicKey, privateKey } = crypto.generateKeyPairSync('rsa', {
          modulusLength: 2048,
          publicKeyEncoding: { type: 'spki', format: 'pem' },
          privateKeyEncoding: { type: 'pkcs1', format: 'pem' },
        });
        fs.writeFileSync(csrFilePath, await generateCsr({ publicKey, privateKey }, '1.2.3.4'));

        const result = await validateZeroSslCertificate(config, expirationDays);

        expect(result.data.isCsrFilePresent).to.be.false();
      });

      // A private key in the request file would go to ZeroSSL with the
      // request, and into a doctor report.
      it('should not count a request that holds a private key as the request', async () => {
        fs.writeFileSync(csrFilePath, `${request}${pair.keyPem}`);

        const result = await validateZeroSslCertificate(config, expirationDays);

        expect(result.data.isCsrFilePresent).to.be.false();
      });

      it('should not renew with a request that holds a private key', async function it() {
        fs.writeFileSync(csrFilePath, `${request}${pair.keyPem}`);
        getCertificate.resolves({
          common_name: '1.2.3.4',
          status: 'issued',
          isExpiredInDays: this.sinon.stub().returns(true),
        });

        const result = await validateZeroSslCertificate(config, expirationDays);

        expect(result.error).to.equal(ERRORS.CSR_FILE_IS_NOT_PRESENT);
        expect(result.data.csr).to.be.undefined();
        expect(JSON.stringify(result.data)).to.not.include('PRIVATE KEY');
      });

      // ZeroSSL gets the request that was checked, not what else the file
      // holds, nor what the file holds after the lookup.
      it('should renew with the request alone', async function it() {
        fs.writeFileSync(csrFilePath, `${request}${pair.pem}`);
        getCertificate.resolves({
          common_name: '1.2.3.4',
          status: 'issued',
          isExpiredInDays: this.sinon.stub().returns(true),
        });

        const result = await validateZeroSslCertificate(config, expirationDays);

        expect(result.error).to.equal(ERRORS.CERTIFICATE_EXPIRES_SOON);
        expect(result.data.csr).to.equal(request);
      });

      it('should renew with the request it checked', async function it() {
        fs.writeFileSync(csrFilePath, request);
        getCertificate.callsFake(async () => {
          fs.writeFileSync(csrFilePath, `${request}${pair.keyPem}`);

          return {
            common_name: '1.2.3.4',
            status: 'issued',
            isExpiredInDays: this.sinon.stub().returns(true),
          };
        });

        const result = await validateZeroSslCertificate(config, expirationDays);

        expect(result.error).to.equal(ERRORS.CERTIFICATE_EXPIRES_SOON);
        expect(result.data.csr).to.equal(request);
      });

      // Opening a named pipe without a writer never returns, and a renewal
      // has nothing to send from one.
      it('should not open a named pipe where the request goes', async function it() {
        fs.rmSync(csrFilePath);
        execFileSync('mkfifo', [csrFilePath]);
        getCertificate.resolves({
          common_name: '1.2.3.4',
          status: 'issued',
          isExpiredInDays: this.sinon.stub().returns(true),
        });

        const result = await validateZeroSslCertificate(config, expirationDays);

        expect(result.error).to.equal(ERRORS.CSR_FILE_IS_NOT_PRESENT);
        expect(result.data.csr).to.be.undefined();
      });

      // ZeroSSL gets the bytes on disk, so they are what has to be a request
      // and carry the signature.
      describe('as PKCS #10 lays it out', () => {
        /**
         * @param {string} pem
         * @param {function(Buffer): Buffer} change
         * @return {string}
         */
        function changeRequest(pem, change) {
          const [block] = forge.pem.decode(pem);
          const der = change(Buffer.from(block.body, 'binary'));

          return forge.pem.encode({ type: block.type, body: der.toString('binary') });
        }

        it('should not count a request whose outer sequence is a set', async () => {
          fs.writeFileSync(csrFilePath, changeRequest(
            createCertificateRequest(pair.keyPem),
            (der) => Buffer.concat([Buffer.from([0x31]), der.subarray(1)]),
          ));

          const result = await validateZeroSslCertificate(config, expirationDays);

          expect(result.data.isCsrFilePresent).to.be.false();
        });

        it('should not count a request whose algorithm is not an object identifier', async () => {
          // sha256WithRSAEncryption, after the subject's rsaEncryption key
          const oid = Buffer.from('06092a864886f70d01010b', 'hex');

          fs.writeFileSync(csrFilePath, changeRequest(
            createCertificateRequest(pair.keyPem),
            (der) => {
              const changed = Buffer.from(der);
              changed[der.indexOf(oid)] = 0x04;

              return changed;
            },
          ));

          const result = await validateZeroSslCertificate(config, expirationDays);

          expect(result.data.isCsrFilePresent).to.be.false();
        });

        it('should not count a request with parameters its algorithm does not take', async () => {
          fs.writeFileSync(csrFilePath, createCertificateRequest(pair.keyPem, {
            algorithm: {
              oid: '1.2.840.113549.1.1.11',
              hash: 'sha256',
              parameters: Buffer.from('0403010203', 'hex'),
            },
          }));

          const result = await validateZeroSslCertificate(config, expirationDays);

          expect(result.data.isCsrFilePresent).to.be.false();
        });

        it('should not count a request whose version is not in its shortest form', async () => {
          fs.writeFileSync(csrFilePath, createCertificateRequest(pair.keyPem, {
            version: Buffer.from('02020000', 'hex'),
          }));

          const result = await validateZeroSslCertificate(config, expirationDays);

          expect(result.data.isCsrFilePresent).to.be.false();
        });

        // An attribute of a type nobody here knows, whose value starts with a
        // zero byte. OpenSSL verifies it; node-forge drops that byte when it
        // encodes the request again.
        it('should count a request with an attribute only its bytes describe', async () => {
          const ecKeyPem = crypto.generateKeyPairSync('ec', { namedCurve: 'prime256v1' })
            .privateKey.export({ type: 'sec1', format: 'pem' });
          fs.writeFileSync(path.join(sslConfigDir, 'private.key'), ecKeyPem);
          fs.writeFileSync(csrFilePath, createCertificateRequest(ecKeyPem, {
            attributes: Buffer.from('301106092b06010401868d1f01310482020001', 'hex'),
          }));

          const result = await validateZeroSslCertificate(config, expirationDays);

          expect(result.data.isCsrFilePresent).to.be.true();
        });
      });

      // OpenSSL signs requests with these when asked, and Node checks them.
      [
        ['RSA', 'SHA3-256', '2.16.840.1.101.3.4.3.14', 'sha3-256', Buffer.alloc(0)],
        ['RSA', 'SHA-512/256', '1.2.840.113549.1.1.16', 'sha512-256', Buffer.from('0500', 'hex')],
        ['EC', 'SHA3-256', '2.16.840.1.101.3.4.3.10', 'sha3-256', Buffer.alloc(0)],
      ].forEach(([keyType, hashName, oid, hash, parameters]) => {
        it(`should count a request for an ${keyType} key signed with ${hashName}`, async () => {
          const keyPem = keyType === 'RSA'
            ? pair.keyPem
            : crypto.generateKeyPairSync('ec', { namedCurve: 'prime256v1' })
              .privateKey.export({ type: 'sec1', format: 'pem' });
          fs.writeFileSync(path.join(sslConfigDir, 'private.key'), keyPem);
          fs.writeFileSync(csrFilePath, createCertificateRequest(keyPem, {
            algorithm: { oid, hash, parameters },
          }));

          const result = await validateZeroSslCertificate(config, expirationDays);

          expect(result.data.isCsrFilePresent).to.be.true();
        });
      });

      // ZeroSSL issues for EC keys as well as RSA ones, and a pair placed by
      // hand renews like one Dashmate made.
      describe('for an EC key', () => {
        let ecKeyPem;

        beforeEach(() => {
          ecKeyPem = crypto.generateKeyPairSync('ec', { namedCurve: 'prime256v1' })
            .privateKey.export({ type: 'sec1', format: 'pem' });
          fs.writeFileSync(path.join(sslConfigDir, 'private.key'), ecKeyPem);
        });

        it('should count a request for the key on disk as the request', async () => {
          fs.writeFileSync(csrFilePath, createCertificateRequest(ecKeyPem));

          const result = await validateZeroSslCertificate(config, expirationDays);

          expect(result.data.isCsrFilePresent).to.be.true();
        });

        it('should not count a request for another key as the request', async () => {
          const otherKeyPem = crypto.generateKeyPairSync('ec', { namedCurve: 'prime256v1' })
            .privateKey.export({ type: 'sec1', format: 'pem' });
          fs.writeFileSync(csrFilePath, createCertificateRequest(otherKeyPem));

          const result = await validateZeroSslCertificate(config, expirationDays);

          expect(result.data.isCsrFilePresent).to.be.false();
        });

        it('should not count a request its key did not sign as the request', async () => {
          const otherKeyPem = crypto.generateKeyPairSync('ec', { namedCurve: 'prime256v1' })
            .privateKey.export({ type: 'sec1', format: 'pem' });
          fs.writeFileSync(csrFilePath, createCertificateRequest(ecKeyPem, {
            signingKeyPem: otherKeyPem,
          }));

          const result = await validateZeroSslCertificate(config, expirationDays);

          expect(result.data.isCsrFilePresent).to.be.false();
        });

        it('should renew with the request on disk', async function it() {
          const ecRequest = createCertificateRequest(ecKeyPem);
          fs.writeFileSync(csrFilePath, ecRequest);
          getCertificate.resolves({
            common_name: '1.2.3.4',
            status: 'issued',
            isExpiredInDays: this.sinon.stub().returns(true),
          });

          const result = await validateZeroSslCertificate(config, expirationDays);

          expect(result.error).to.equal(ERRORS.CERTIFICATE_EXPIRES_SOON);
          expect(result.data.csr).to.equal(ecRequest);
        });
      });

      // A renewal needs a request it can send; without one only a new
      // certificate for a new key replaces the expiring one.
      it('should not renew with a request cut short', async function it() {
        fs.writeFileSync(csrFilePath, request.slice(0, request.length / 2));
        getCertificate.resolves({
          common_name: '1.2.3.4',
          status: 'issued',
          isExpiredInDays: this.sinon.stub().returns(true),
        });

        const result = await validateZeroSslCertificate(config, expirationDays);

        expect(result.error).to.equal(ERRORS.CSR_FILE_IS_NOT_PRESENT);
        expect(result.data.csr).to.be.undefined();
      });
    });
  });
});
