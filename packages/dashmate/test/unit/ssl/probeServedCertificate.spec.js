import net from 'node:net';
import tls from 'node:tls';
import probeServedCertificate, { STATE } from '../../../src/ssl/probeServedCertificate.js';
import createCertificateForTest from '../../../src/test/createCertificateForTest.js';

const EXTERNAL_IP = '127.0.0.1';

describe('probeServedCertificate', () => {
  const servers = [];
  const sockets = [];

  /**
   * Track every accepted connection so a server can be closed without waiting on one that the
   * test deliberately left open.
   *
   * @param {Server} server
   * @return {Server}
   */
  function track(server) {
    server.on('connection', (socket) => sockets.push(socket));
    server.on('secureConnection', (socket) => sockets.push(socket));

    servers.push(server);

    return server;
  }

  /**
   * @param {Object} tlsOptions
   * @return {Promise<number>} listening port
   */
  async function listenTls(tlsOptions) {
    const server = track(tls.createServer(tlsOptions, (socket) => socket.end()));

    await new Promise((resolve) => {
      server.listen(0, '127.0.0.1', resolve);
    });

    return server.address().port;
  }

  afterEach(async () => {
    sockets.splice(0).forEach((socket) => socket.destroy());

    await Promise.all(servers.splice(0).map((server) => new Promise((resolve) => {
      server.close(resolve);
    })));
  });

  it('should report the certificate the server actually serves', async () => {
    const { cert, key } = createCertificateForTest({ days: 30 });
    const port = await listenTls({ cert, key });

    const result = await probeServedCertificate({ host: '127.0.0.1', port, externalIp: EXTERNAL_IP });

    expect(result.state).to.equal(STATE.SERVED);
    expect(result.certificate.fingerprint256).to.match(/^[0-9A-F]{2}(:[0-9A-F]{2})+$/);
    expect(new Date(result.certificate.validTo).getTime()).to.be.greaterThan(Date.now());
  });

  it('should complete the handshake and report an expired certificate', async () => {
    const { cert, key } = createCertificateForTest({ days: -5 });
    const port = await listenTls({ cert, key });

    const result = await probeServedCertificate({ host: '127.0.0.1', port, externalIp: EXTERNAL_IP });

    expect(result.state).to.equal(STATE.SERVED);
    expect(new Date(result.certificate.validTo).getTime()).to.be.lessThan(Date.now());
  });

  it('should not fail identity for a certificate naming the external IP rather than the probed address', async () => {
    // The gateway is reached on loopback but its certificate names the node's public address.
    // Judging identity against the dialled address would fail every healthy node.
    const { cert, key } = createCertificateForTest({ ip: '198.51.100.7' });
    const port = await listenTls({ cert, key });

    const result = await probeServedCertificate({
      host: '127.0.0.1',
      port,
      externalIp: '198.51.100.7',
    });

    expect(result.state).to.equal(STATE.SERVED);
    expect(result.identityVerified).to.be.true();
  });

  it('should report an identity mismatch against the external IP', async () => {
    const { cert, key } = createCertificateForTest({ ip: '203.0.113.9' });
    const port = await listenTls({ cert, key });

    const result = await probeServedCertificate({
      host: '127.0.0.1',
      port,
      externalIp: '198.51.100.7',
    });

    expect(result.state).to.equal(STATE.SERVED);
    expect(result.identityVerified).to.be.false();
  });

  it('should report identity separately from the chain verdict when both fail', async () => {
    // The socket surfaces only the first verification failure, so an expired certificate that
    // also names the wrong address would otherwise hide the mismatch until the expiry was fixed.
    const { cert, key } = createCertificateForTest({ ip: '203.0.113.9', days: -5 });
    const port = await listenTls({ cert, key });

    const result = await probeServedCertificate({
      host: '127.0.0.1',
      port,
      externalIp: '198.51.100.7',
    });

    expect(result.chainVerified).to.be.false();
    expect(result.identityVerified).to.be.false();
  });

  it('should report unreachable when nothing is listening', async () => {
    const result = await probeServedCertificate({
      host: '127.0.0.1',
      // Port 1 is privileged and unused, so the connection is refused rather than answered
      port: 1,
      externalIp: EXTERNAL_IP,
    });

    expect(result.state).to.equal(STATE.UNREACHABLE);
    expect(result.certificate).to.be.undefined();
  });

  it('should give up on a peer that accepts the connection and never completes the handshake', async () => {
    const server = track(net.createServer(() => {}));

    await new Promise((resolve) => {
      server.listen(0, '127.0.0.1', resolve);
    });

    const result = await probeServedCertificate({
      host: '127.0.0.1',
      port: server.address().port,
      externalIp: EXTERNAL_IP,
      timeout: 300,
    });

    expect(result.state).to.equal(STATE.UNREACHABLE);
    expect(result.reason).to.equal('ETIMEDOUT');
  });

  it('should give up on a peer that trickles data without completing the handshake', async () => {
    // The socket's own timeout resets on every byte received, so a slow drip would keep the
    // probe alive forever if the deadline were not independent of it.
    const server = track(net.createServer((socket) => {
      const interval = setInterval(() => socket.write('\0'), 50);
      socket.on('close', () => clearInterval(interval));
      socket.on('error', () => clearInterval(interval));
    }));

    await new Promise((resolve) => {
      server.listen(0, '127.0.0.1', resolve);
    });

    const result = await probeServedCertificate({
      host: '127.0.0.1',
      port: server.address().port,
      externalIp: EXTERNAL_IP,
      timeout: 400,
    });

    expect(result.state).to.equal(STATE.UNREACHABLE);
  });
});
