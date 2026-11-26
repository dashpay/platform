import fs from 'fs';
import os from 'os';
import path from 'path';
import HomeDir from '../../../src/config/HomeDir.js';
import getBaseConfigFactory from '../../../configs/defaults/getBaseConfigFactory.js';
import installCertificateFilesTaskFactory from '../../../src/listr/tasks/ssl/installCertificateFilesTaskFactory.js';
import saveCertificateTaskFactory from '../../../src/listr/tasks/ssl/saveCertificateTask.js';
import { readCertificateFilePair } from '../../../src/listr/prompts/validators/validateCertificateFilePaths.js';
import RenewalRecordRepository from '../../../src/ssl/renewalRecord/RenewalRecordRepository.js';
import { issueCertificate } from '../../../src/test/certificateFixtures.js';

describe('installCertificateFilesTask', () => {
  let homeDir;
  let sourceDir;
  let config;
  let installCertificateFilesTask;
  let validated;
  let other;
  let form;

  before(function before() {
    // RSA key generation is slow in pure JS.
    this.timeout(30000);
    validated = issueCertificate({ ip: '1.2.3.4' });
    other = issueCertificate({ ip: '1.2.3.4' });
  });

  beforeEach(() => {
    homeDir = HomeDir.createTemp();
    sourceDir = fs.mkdtempSync(path.join(os.tmpdir(), 'dashmate-certificate-source-'));
    config = getBaseConfigFactory(homeDir)();

    form = {
      chainFilePath: path.join(sourceDir, 'chain.pem'),
      privateFilePath: path.join(sourceDir, 'key.pem'),
    };
    fs.writeFileSync(form.chainFilePath, validated.pem);
    fs.writeFileSync(form.privateFilePath, validated.keyPem, { mode: 0o600 });

    installCertificateFilesTask = installCertificateFilesTaskFactory(
      saveCertificateTaskFactory(homeDir, new RenewalRecordRepository(homeDir)),
    );
  });

  afterEach(() => {
    homeDir.remove();
    fs.rmSync(sourceDir, { recursive: true, force: true });
  });

  /**
   * @return {{ bundle: string, key: string }} what the gateway reads
   */
  function installed() {
    const sslDir = homeDir.joinPath(config.getName(), 'platform', 'gateway', 'ssl');

    return {
      bundle: fs.readFileSync(path.join(sslDir, 'bundle.crt'), 'utf8'),
      key: fs.readFileSync(path.join(sslDir, 'private.key'), 'utf8'),
    };
  }

  // Platform setup checks the files before it starts Core and installs them
  // after Core has synced, which can take hours. A source replaced meanwhile
  // must not be what reaches the gateway.
  it('should install the bytes that were checked, not the files as they are now', async () => {
    const pair = readCertificateFilePair(form);

    fs.writeFileSync(form.chainFilePath, other.pem);
    fs.writeFileSync(form.privateFilePath, other.keyPem);

    await installCertificateFilesTask(config).run({
      fileCertificateProviderForm: form,
      validatedCertificatePair: pair,
    });

    expect(installed()).to.deep.equal({ bundle: validated.pem, key: validated.keyPem });
  });

  // Given paths only, it reads each file once and installs what it checked.
  it('should check and install the files given by path', async () => {
    await installCertificateFilesTask(config).run({ fileCertificateProviderForm: form });

    expect(installed()).to.deep.equal({ bundle: validated.pem, key: validated.keyPem });
  });

  it('should refuse a pair given by path that does not match, installing nothing', async () => {
    fs.writeFileSync(form.privateFilePath, other.keyPem);

    await expect(installCertificateFilesTask(config).run({ fileCertificateProviderForm: form }))
      .to.be.rejectedWith('do not match');

    const sslDir = homeDir.joinPath(config.getName(), 'platform', 'gateway', 'ssl');
    expect(fs.existsSync(path.join(sslDir, 'bundle.crt'))).to.be.false();
  });
});

describe('readCertificateFilePair', () => {
  let dir;
  let pair;
  let form;

  before(function before() {
    this.timeout(30000);
    pair = issueCertificate({ ip: '1.2.3.4' });
  });

  beforeEach(() => {
    dir = fs.mkdtempSync(path.join(os.tmpdir(), 'dashmate-certificate-pair-'));
    form = {
      chainFilePath: path.join(dir, 'chain.pem'),
      privateFilePath: path.join(dir, 'key.pem'),
    };
    fs.writeFileSync(form.chainFilePath, pair.pem);
    fs.writeFileSync(form.privateFilePath, pair.keyPem);
  });

  afterEach(() => {
    fs.rmSync(dir, { recursive: true, force: true });
  });

  it('should return the contents it checked', () => {
    expect(readCertificateFilePair(form)).to.deep.equal({
      certificateFile: pair.pem,
      privateKeyFile: pair.keyPem,
    });
  });

  // The gateway loads every block of the bundle, so a damaged one after the
  // leaf keeps it from starting, and is refused before Core is started.
  it('should refuse a bundle with a damaged block after the leaf', () => {
    fs.writeFileSync(form.chainFilePath, `${pair.pem}-----BEGIN CERTIFICATE-----\nAAAA\n`);

    expect(() => readCertificateFilePair(form)).to.throw('cannot be used');
  });

  // Reading a FIFO or a device would wait or never end.
  it('should refuse a path that is not a regular file', () => {
    expect(() => readCertificateFilePair({ ...form, chainFilePath: dir }))
      .to.throw('the certificate and key must be regular files');
  });
});
