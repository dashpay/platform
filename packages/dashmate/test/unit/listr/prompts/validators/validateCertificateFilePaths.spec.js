import fs from 'fs';
import os from 'os';
import path from 'path';
import validateCertificateFilePaths from '../../../../../src/listr/prompts/validators/validateCertificateFilePaths.js';
import validateFileExists from '../../../../../src/listr/prompts/validators/validateFileExists.js';
import { issueCertificate } from '../../../../../src/test/certificateFixtures.js';

describe('validateCertificateFilePaths', () => {
  let dir;
  let chainFilePath;
  let privateFilePath;

  before(function before() {
    // RSA key generation is slow in pure JS.
    this.timeout(30000);
  });

  beforeEach(() => {
    dir = fs.mkdtempSync(path.join(os.tmpdir(), 'dashmate-certificate-paths-'));

    const { pem, keyPem } = issueCertificate();
    chainFilePath = path.join(dir, 'bundle.crt');
    privateFilePath = path.join(dir, 'private.key');
    fs.writeFileSync(chainFilePath, pem);
    fs.writeFileSync(privateFilePath, keyPem);
  });

  afterEach(() => {
    fs.chmodSync(dir, 0o700);
    fs.rmSync(dir, { recursive: true, force: true });
  });

  it('should accept a matching certificate and key', () => {
    expect(validateCertificateFilePaths({ chainFilePath, privateFilePath })).to.be.true();
  });

  // Setup only reads them, and writes copies into the gateway's directory. A
  // key hardened to 0400, or files on a read-only secrets mount, are what an
  // operator should be able to give it.
  it('should accept files it can read but not write', function it() {
    if (process.getuid?.() === 0) {
      // root writes whatever the mode says
      this.skip();
    }

    fs.chmodSync(privateFilePath, 0o400);
    fs.chmodSync(chainFilePath, 0o444);

    expect(validateFileExists(privateFilePath)).to.be.true();
    expect(validateFileExists(chainFilePath)).to.be.true();
    expect(validateCertificateFilePaths({ chainFilePath, privateFilePath })).to.be.true();
  });

  it('should refuse a file it cannot read', function it() {
    if (process.getuid?.() === 0) {
      this.skip();
    }

    fs.chmodSync(privateFilePath, 0o200);

    expect(validateFileExists(privateFilePath)).to.be.false();
    expect(validateCertificateFilePaths({ chainFilePath, privateFilePath }))
      .to.equal('certificate key file path is not valid');
  });
});
