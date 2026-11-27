import fs from 'fs';
import os from 'os';
import path from 'path';
import { Listr } from 'listr2';
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

  // The form's own check reads the files when it is submitted. A file
  // replaced before the form returns must not be what is installed.
  it('should install the pair the form accepted, not the files as they are when it returns', async () => {
    const enquirer = {
      on: () => {},
      prompt: async (options) => {
        const [prompt] = [].concat(options);

        expect(prompt.validate(form)).to.be.true();

        fs.writeFileSync(form.chainFilePath, other.pem);
        fs.writeFileSync(form.privateFilePath, other.keyPem);

        return { default: form };
      },
    };

    const tasks = new Listr([{
      task: () => installCertificateFilesTask(config, { interactive: true }),
    }], { renderer: 'silent' });
    tasks.options.injectWrapper = { enquirer };

    await tasks.run({});

    expect(installed()).to.deep.equal({ bundle: validated.pem, key: validated.keyPem });
  });

  // PEM allows text outside its blocks, in any encoding: the gateway gets the
  // file as it is.
  it('should install the files byte for byte', async () => {
    const bytes = Buffer.concat([Buffer.from('Caf'), Buffer.from([0xe9]), Buffer.from('\n'), Buffer.from(validated.pem)]);
    fs.writeFileSync(form.chainFilePath, bytes);

    await installCertificateFilesTask(config).run({ fileCertificateProviderForm: form });

    const sslDir = homeDir.joinPath(config.getName(), 'platform', 'gateway', 'ssl');
    expect(fs.readFileSync(path.join(sslDir, 'bundle.crt')).equals(bytes)).to.be.true();
  });

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

  // The bundle is written at the umask, readable by other users, while the
  // key is kept 0600. A combined PEM given as the chain would put a second
  // copy of the key in the bundle.
  it('should refuse a chain file that holds a private key, installing nothing', async () => {
    fs.writeFileSync(form.chainFilePath, `${validated.pem}${validated.keyPem}`);

    await expect(installCertificateFilesTask(config).run({ fileCertificateProviderForm: form }))
      .to.be.rejectedWith('holds a private key');

    const sslDir = homeDir.joinPath(config.getName(), 'platform', 'gateway', 'ssl');
    expect(fs.existsSync(path.join(sslDir, 'bundle.crt'))).to.be.false();
    expect(fs.existsSync(path.join(sslDir, 'private.key'))).to.be.false();
  });

  it('should keep the form open on a chain file that holds a private key', async () => {
    fs.writeFileSync(form.chainFilePath, `${validated.pem}${validated.keyPem}`);
    let message;

    const enquirer = {
      on: () => {},
      prompt: async (options) => {
        const [prompt] = [].concat(options);

        message = prompt.validate(form);

        throw new Error('closed');
      },
    };

    const tasks = new Listr([{
      task: () => installCertificateFilesTask(config, { interactive: true }),
    }], { renderer: 'silent' });
    tasks.options.injectWrapper = { enquirer };

    await expect(tasks.run({})).to.be.rejectedWith('closed');

    expect(message).to.include('holds a private key');
    const sslDir = homeDir.joinPath(config.getName(), 'platform', 'gateway', 'ssl');
    expect(fs.existsSync(path.join(sslDir, 'bundle.crt'))).to.be.false();
    expect(fs.existsSync(path.join(sslDir, 'private.key'))).to.be.false();
  });

  it('should refuse a pair given by path that does not match, installing nothing', async () => {
    fs.writeFileSync(form.privateFilePath, other.keyPem);

    await expect(installCertificateFilesTask(config).run({ fileCertificateProviderForm: form }))
      .to.be.rejectedWith('do not match');

    const sslDir = homeDir.joinPath(config.getName(), 'platform', 'gateway', 'ssl');
    expect(fs.existsSync(path.join(sslDir, 'bundle.crt'))).to.be.false();
  });

  // Platform setup checks the pair before it starts Core and installs it
  // after Core has synced, which can outlast what was left of its validity.
  it('should refuse a pair checked in advance that expired before it is installed', async function it() {
    const pair = readCertificateFilePair(form);

    this.sinon.useFakeTimers({ now: Date.now() + 31 * 24 * 60 * 60 * 1000, toFake: ['Date'] });

    await expect(installCertificateFilesTask(config).run({
      fileCertificateProviderForm: form,
      validatedCertificatePair: pair,
    })).to.be.rejectedWith('expired on');

    const sslDir = homeDir.joinPath(config.getName(), 'platform', 'gateway', 'ssl');
    expect(fs.existsSync(path.join(sslDir, 'bundle.crt'))).to.be.false();
    expect(fs.existsSync(path.join(sslDir, 'private.key'))).to.be.false();
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

  it('should return the contents it checked, and the key file\'s mode as it was read', () => {
    fs.chmodSync(form.privateFilePath, 0o640);

    const { certificateFile, privateKeyFile, privateKeyStats } = readCertificateFilePair(form);

    expect(certificateFile.equals(Buffer.from(pair.pem))).to.be.true();
    expect(privateKeyFile.equals(Buffer.from(pair.keyPem))).to.be.true();

    // Windows reports 0666 for any writable file
    if (process.platform !== 'win32') {
      // eslint-disable-next-line no-bitwise
      expect(privateKeyStats.mode & 0o777).to.equal(0o640);
    }
  });

  // The gateway loads every block of the bundle, so a damaged one after the
  // leaf keeps it from starting, and is refused before Core is started.
  it('should refuse a bundle with a damaged block after the leaf', () => {
    fs.writeFileSync(form.chainFilePath, `${pair.pem}-----BEGIN CERTIFICATE-----\nAAAA\n`);

    expect(() => readCertificateFilePair(form)).to.throw('cannot be used');
  });

  // Clients refuse a certificate outside its validity window on either side,
  // and `update` refuses to go on with one.
  it('should refuse an expired certificate', () => {
    fs.writeFileSync(form.chainFilePath, issueCertificate({ ip: '1.2.3.4', keys: pair.keys, days: -1 }).pem);

    expect(() => readCertificateFilePair(form)).to.throw('The certificate expired on');
  });

  it('should refuse a certificate that is not valid yet', () => {
    fs.writeFileSync(
      form.chainFilePath,
      issueCertificate({ ip: '1.2.3.4', keys: pair.keys, startsInDays: 2 }).pem,
    );

    expect(() => readCertificateFilePair(form)).to.throw('The certificate is not valid until');
  });

  // Any private key block counts, whatever its type or form: the chain is
  // installed readable by other users.
  [
    ['the key it pairs with', (key) => key],
    ['a PKCS #8 key', () => '-----BEGIN PRIVATE KEY-----\nAAAA\n-----END PRIVATE KEY-----\n'],
    ['an EC key', () => '-----BEGIN EC PRIVATE KEY-----\nAAAA\n-----END EC PRIVATE KEY-----\n'],
    ['an encrypted key', () => '-----BEGIN ENCRYPTED PRIVATE KEY-----\nAAAA\n-----END ENCRYPTED PRIVATE KEY-----\n'],
    ['an indented key', () => '  -----BEGIN PRIVATE KEY-----\nAAAA\n'],
    ['an OpenPGP private key', () => '-----BEGIN PGP PRIVATE KEY BLOCK-----\n\nAAAA\n-----END PGP PRIVATE KEY BLOCK-----\n'],
  ].forEach(([description, keyBlock]) => {
    it(`should refuse a chain file that holds ${description}`, () => {
      fs.writeFileSync(form.chainFilePath, `${pair.pem}${keyBlock(pair.keyPem)}`);

      expect(() => readCertificateFilePair(form)).to.throw(
        'the certificate chain file holds a private key',
      );
    });
  });

  // Reading a FIFO or a device would wait or never end.
  it('should refuse a path that is not a regular file', () => {
    expect(() => readCertificateFilePair({ ...form, chainFilePath: dir }))
      .to.throw('the certificate and key must be regular files');
  });
});
