import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import tls from 'node:tls';
import getBaseConfigFactory from '../../../configs/defaults/getBaseConfigFactory.js';
import probeGatewayCertificate, {
  waitForGatewayToServeBundle,
} from '../../../src/ssl/probeGatewayCertificate.js';
import { issueCertificate } from '../../../src/test/certificateFixtures.js';

describe('probeGatewayCertificate', () => {
  let server;
  let config;
  let directory;
  let bundleFilePath;
  let served;

  before(function before() {
    // RSA key generation is slow in pure JS.
    this.timeout(30000);
    served = issueCertificate({ ip: '127.0.0.1' });
  });

  beforeEach(async () => {
    server = tls.createServer({ cert: served.pem, key: served.keyPem }, (socket) => socket.end());

    await new Promise((resolve) => {
      server.listen(0, '127.0.0.1', resolve);
    });

    config = getBaseConfigFactory()();
    config.set('externalIp', '127.0.0.1');
    config.set('platform.gateway.listeners.dapiAndDrive.host', '0.0.0.0');
    config.set('platform.gateway.listeners.dapiAndDrive.port', server.address().port);

    directory = fs.mkdtempSync(path.join(os.tmpdir(), 'dashmate-probe-'));
    bundleFilePath = path.join(directory, 'bundle.crt');
  });

  afterEach(async () => {
    await new Promise((resolve) => {
      server.close(resolve);
    });

    fs.rmSync(directory, { recursive: true, force: true });
  });

  it('should match the bundle on disk the gateway serves', async () => {
    fs.writeFileSync(bundleFilePath, served.pem);

    const result = await probeGatewayCertificate(config, bundleFilePath);

    expect(result.state).to.equal('served');
    expect(result.matchesOnDisk).to.be.true();
  });

  // What a gateway does after a bundle was replaced by a new file: it keeps
  // the file it mounted, so it serves another certificate than the one on
  // disk.
  it('should not match a bundle on disk the gateway does not serve', async function it() {
    this.timeout(30000);
    fs.writeFileSync(bundleFilePath, issueCertificate({ ip: '127.0.0.1' }).pem);

    const result = await probeGatewayCertificate(config, bundleFilePath);

    expect(result.matchesOnDisk).to.be.false();
    expect(result.onDisk.fingerprint256).to.not.equal(result.certificate.fingerprint256);
  });
});

describe('waitForGatewayToServeBundle', () => {
  // Right after the signal the previous Envoy still answers with the
  // certificate it loaded, so a first mismatch is not the verdict.
  it('should keep probing until the gateway serves the bundle', async function it() {
    const probe = this.sinon.stub();
    probe.onCall(0).resolves({ state: 'served', matchesOnDisk: false });
    probe.onCall(1).resolves({ state: 'unreachable', reason: 'ECONNRESET' });
    probe.onCall(2).resolves({ state: 'served', matchesOnDisk: true });

    const result = await waitForGatewayToServeBundle({}, '/bundle.crt', {
      timeout: 1000, interval: 1, probe,
    });

    expect(result.matchesOnDisk).to.be.true();
    expect(probe).to.have.been.calledThrice();
  });

  it('should return the last answer once the reload has had its time', async function it() {
    const clock = this.sinon.useFakeTimers();
    const probe = this.sinon.stub().resolves({ state: 'served', matchesOnDisk: false });

    const waiting = waitForGatewayToServeBundle({}, '/bundle.crt', {
      timeout: 3000, interval: 1000, probe,
    });
    await clock.tickAsync(10000);
    const result = await waiting;

    expect(result.matchesOnDisk).to.be.false();
    expect(probe).to.have.callCount(3);
  });

  // A probe has its own five-second limit, so one started near the end
  // would run past the time the reload is allowed.
  it('should not let a probe run past the deadline', async function it() {
    const clock = this.sinon.useFakeTimers();
    const endsAt = [];
    const probe = this.sinon.stub().callsFake(async (config, bundleFilePath, { timeout }) => {
      endsAt.push(Date.now() + timeout);

      await new Promise((resolve) => {
        setTimeout(resolve, timeout);
      });

      return { state: 'unreachable', reason: 'ETIMEDOUT' };
    });

    const waiting = waitForGatewayToServeBundle({}, '/bundle.crt', {
      timeout: 7000, interval: 1000, probe,
    });
    await clock.tickAsync(20000);
    await waiting;

    expect(endsAt).to.have.lengthOf(2);
    expect(Math.max(...endsAt)).to.be.at.most(7000);
  });
});
