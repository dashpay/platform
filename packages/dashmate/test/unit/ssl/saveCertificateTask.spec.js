import fs from 'fs';
import path from 'path';
import graceful from 'node-graceful';
import HomeDir from '../../../src/config/HomeDir.js';
import getBaseConfigFactory from '../../../configs/defaults/getBaseConfigFactory.js';
import saveCertificateTaskFactory from '../../../src/listr/tasks/ssl/saveCertificateTask.js';

describe('saveCertificateTaskFactory', () => {
  let homeDir;
  let config;
  let certificatesDir;
  let certificatePath;
  let keyPath;
  let previousUmask;

  beforeEach(() => {
    previousUmask = process.umask(0o022);
    homeDir = HomeDir.createTemp();
    config = getBaseConfigFactory(homeDir)();
    config.set('platform.gateway.ssl.enabled', true);
    certificatesDir = homeDir.joinPath(
      config.getName(),
      'platform',
      'gateway',
      'ssl',
    );
    certificatePath = path.join(certificatesDir, 'bundle.crt');
    keyPath = path.join(certificatesDir, 'private.key');
  });

  afterEach(() => {
    process.umask(previousUmask);
    homeDir.remove();
  });

  async function savePair() {
    const task = saveCertificateTaskFactory(homeDir)(config);

    await task.run({
      certificateFile: 'new-certificate',
      privateKeyFile: 'new-key',
    });
  }

  function mode(filePath) {
    // eslint-disable-next-line no-bitwise
    return fs.statSync(filePath).mode & 0o777;
  }

  it('should create a private key with mode 0600', async () => {
    await savePair();

    expect(mode(keyPath)).to.equal(0o600);
  });

  it('should preserve existing certificate and key modes when replacing them', async () => {
    fs.mkdirSync(certificatesDir, { recursive: true });
    fs.writeFileSync(certificatePath, 'old-certificate');
    fs.writeFileSync(keyPath, 'old-key');
    fs.chmodSync(certificatePath, 0o640);
    fs.chmodSync(keyPath, 0o600);

    await savePair();

    expect(mode(certificatePath)).to.equal(0o640);
    expect(mode(keyPath)).to.equal(0o600);
  });

  // A node set up before Dashmate chose a mode carries a private key readable
  // by every local account. Renewal is the only thing that touches the file
  // again, so preserving what it finds would leave that key exposed for the
  // life of the node.
  it('should tighten a private key left group- and world-readable by an older version', async () => {
    fs.mkdirSync(certificatesDir, { recursive: true });
    fs.writeFileSync(certificatePath, 'old-certificate');
    fs.writeFileSync(keyPath, 'old-key');
    fs.chmodSync(keyPath, 0o644);

    await savePair();

    expect(mode(keyPath)).to.equal(0o600);
  });

  it('should keep a private key mode stricter than Dashmate would choose', async () => {
    fs.mkdirSync(certificatesDir, { recursive: true });
    fs.writeFileSync(certificatePath, 'old-certificate');
    fs.writeFileSync(keyPath, 'old-key');
    fs.chmodSync(keyPath, 0o400);

    await savePair();

    expect(mode(keyPath)).to.equal(0o400);
  });

  it('should restore the previous certificate pair and modes when saving the key fails', async function it() {
    fs.mkdirSync(certificatesDir, { recursive: true });
    fs.writeFileSync(certificatePath, 'old-certificate');
    fs.writeFileSync(keyPath, 'old-key');
    fs.chmodSync(certificatePath, 0o640);
    fs.chmodSync(keyPath, 0o600);

    const originalRenameSync = fs.renameSync.bind(fs);
    this.sinon.stub(fs, 'renameSync').callsFake((source, destination) => {
      if (destination === keyPath) {
        throw new Error('key replace failed');
      }

      return originalRenameSync(source, destination);
    });

    await expect(savePair()).to.be.rejectedWith('key replace failed');

    expect(fs.readFileSync(certificatePath, 'utf8')).to.equal('old-certificate');
    expect(fs.readFileSync(keyPath, 'utf8')).to.equal('old-key');
    expect(mode(certificatePath)).to.equal(0o640);
    expect(mode(keyPath)).to.equal(0o600);
    expect(fs.readdirSync(certificatesDir).filter((name) => name.includes('.tmp-')))
      .to.be.empty();
    expect(config.get('platform.gateway.ssl.enabled')).to.be.true();
  });

  it('should sweep stale certificate temp files before writing', async () => {
    fs.mkdirSync(certificatesDir, { recursive: true });
    fs.writeFileSync(path.join(certificatesDir, 'bundle.crt.tmp-stale'), 'old-certificate');
    fs.writeFileSync(path.join(certificatesDir, 'private.key.tmp-stale'), 'old-key');

    await savePair();

    expect(fs.readdirSync(certificatesDir).filter((name) => name.includes('.tmp-')))
      .to.be.empty();
  });

  it('should remove active certificate temp files from the graceful exit handler', async function it() {
    let exitHandler;
    const unsubscribe = this.sinon.stub();
    this.sinon.stub(graceful, 'on').callsFake((event, handler) => {
      expect(event).to.equal('exit');
      exitHandler = handler;
      return unsubscribe;
    });

    const originalRenameSync = fs.renameSync.bind(fs);
    this.sinon.stub(fs, 'renameSync').callsFake((source, destination) => {
      if (destination === certificatePath) {
        expect(exitHandler).to.be.a('function');
        expect(fs.existsSync(source)).to.be.true();
        exitHandler();
        expect(fs.existsSync(source)).to.be.false();
        throw new Error('exit cleanup observed');
      }

      return originalRenameSync(source, destination);
    });

    await expect(savePair()).to.be.rejectedWith('exit cleanup observed');

    expect(unsubscribe).to.have.been.calledOnce();
    expect(fs.readdirSync(certificatesDir).filter((name) => name.includes('.tmp-')))
      .to.be.empty();
  });
});
