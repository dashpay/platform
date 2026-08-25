import fs from 'fs';
import path from 'path';
import HomeDir from '../../../src/config/HomeDir.js';
import getBaseConfigFactory from '../../../configs/defaults/getBaseConfigFactory.js';
import saveCertificateTaskFactory from '../../../src/listr/tasks/ssl/saveCertificateTask.js';
import { issueCertificate } from '../../../src/test/certificateFixtures.js';

describe('saveCertificateTaskFactory', () => {
  let homeDir;
  let config;
  let certificatesDir;
  let certificatePath;
  let keyPath;
  let previousUmask;
  let pair;

  beforeEach(() => {
    pair = issueCertificate({ ip: '1.2.3.4' });
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

  async function savePair(context = {}) {
    const task = saveCertificateTaskFactory(homeDir)(config);

    await task.run({
      certificateFile: pair.pem,
      privateKeyFile: pair.keyPem,
      ...context,
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
    expect(fs.readFileSync(certificatePath, 'utf8')).to.equal(pair.pem);
    expect(fs.readFileSync(keyPath, 'utf8')).to.equal(pair.keyPem);
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

  // Writing in place needs the owner write bit, so a key hardened to 0400 is
  // loosened for the write. A write that then fails must not leave it that way.
  it('should keep a hardened private key mode when the write fails', async function it() {
    fs.mkdirSync(certificatesDir, { recursive: true });
    fs.writeFileSync(certificatePath, 'old-certificate');
    fs.writeFileSync(keyPath, 'old-key');
    fs.chmodSync(keyPath, 0o400);

    const originalWriteFileSync = fs.writeFileSync.bind(fs);
    this.sinon.stub(fs, 'writeFileSync').callsFake((filePath, data, options) => {
      if (filePath === keyPath) {
        throw new Error('key write failed');
      }

      return originalWriteFileSync(filePath, data, options);
    });

    await expect(savePair()).to.be.rejectedWith('key write failed');

    expect(mode(keyPath)).to.equal(0o400);
  });

  it('should keep a private key mode stricter than Dashmate would choose', async () => {
    fs.mkdirSync(certificatesDir, { recursive: true });
    fs.writeFileSync(certificatePath, 'old-certificate');
    fs.writeFileSync(keyPath, 'old-key');
    fs.chmodSync(keyPath, 0o400);

    await savePair();

    expect(mode(keyPath)).to.equal(0o400);
  });

  // The bundle and the key are two separate in-place writes - in place because
  // the bind mount follows the inode - so a full disk, a failed chmod or a
  // power loss between them leaves a new certificate paired with the old key.
  // With the gateway stopped, as the documented upgrade procedure leaves it,
  // nothing else would notice: the command reports success and the node simply
  // fails to come back up at the next `dashmate start`, a step removed from
  // whatever caused it.
  it('should refuse to report success when the written pair does not match', async function it() {
    const other = issueCertificate({ ip: '1.2.3.4' });

    await expect(savePair({ privateKeyFile: other.keyPem }))
      .to.be.rejectedWith(/do not match/i);
  });

  it('should name the repair when the written pair does not match', async function it() {
    const other = issueCertificate({ ip: '1.2.3.4' });

    const error = await savePair({ privateKeyFile: other.keyPem }).catch((e) => e);

    expect(error.message).to.contain(`--config ${config.getName()}`);
    expect(error.message).to.contain('dashmate ssl obtain');
  });

  // Models the write that fails without throwing: the certificate lands, the
  // key never does, and the old key is left in place.
  it('should catch a key that never reached the disk', async function it() {
    await savePair();

    const renewed = issueCertificate({ ip: '1.2.3.4' });
    const writeFileSync = this.sinon.stub(fs, 'writeFileSync');
    writeFileSync.callThrough();
    writeFileSync.withArgs(keyPath).returns(undefined);

    await expect(savePair({
      certificateFile: renewed.pem,
      privateKeyFile: renewed.keyPem,
    })).to.be.rejectedWith(/do not match/i);
  });
});
