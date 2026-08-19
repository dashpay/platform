import fs from 'fs';
import path from 'path';
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

  // Docker bind-mounts bundle.crt and private.key into the gateway container
  // as individual files, and a file bind mount follows the inode rather than
  // the path. Installing a renewal by writing a replacement file and renaming
  // it over the old one leaves the running container reading the file it
  // mounted at startup, so Envoy keeps serving the previous certificate until
  // it expires and the node goes dark with nothing on disk looking wrong.
  it('should install a renewal into the files the gateway already has mounted', async () => {
    fs.mkdirSync(certificatesDir, { recursive: true });
    fs.writeFileSync(certificatePath, 'old-certificate');
    fs.writeFileSync(keyPath, 'old-key');

    const certificateInode = fs.statSync(certificatePath).ino;
    const keyInode = fs.statSync(keyPath).ino;

    await savePair();

    expect(fs.statSync(certificatePath).ino).to.equal(certificateInode);
    expect(fs.statSync(keyPath).ino).to.equal(keyInode);
    expect(fs.readFileSync(certificatePath, 'utf8')).to.equal('new-certificate');
    expect(fs.readFileSync(keyPath, 'utf8')).to.equal('new-key');
  });

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
});
