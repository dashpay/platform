import fs from 'fs';
import path from 'path';
import HomeDir from '../../../src/config/HomeDir.js';
import getBaseConfigFactory from '../../../configs/defaults/getBaseConfigFactory.js';
import checkGatewayCertificateFactory, {
  CERTIFICATE_REASONS,
  CERTIFICATE_STATUS,
} from '../../../src/ssl/checkGatewayCertificateFactory.js';
import {
  encryptPrivateKey,
  issueCertificate,
  issueChain,
  issueEd25519Certificate,
} from '../../../src/test/certificateFixtures.js';

const EXTERNAL_IP = '1.2.3.4';

describe('checkGatewayCertificateFactory', () => {
  let homeDir;
  let config;
  let sslDir;
  let legoDir;
  let checkGatewayCertificate;

  beforeEach(() => {
    homeDir = HomeDir.createTemp();
    config = getBaseConfigFactory(homeDir)();
    config.set('externalIp', EXTERNAL_IP);
    config.set('platform.gateway.ssl.enabled', true);
    config.set('platform.gateway.ssl.provider', 'letsencrypt');
    config.set('core.masternode.enable', true);

    sslDir = homeDir.joinPath(config.getName(), 'platform', 'gateway', 'ssl');
    legoDir = homeDir.joinPath(config.getName(), 'platform', 'gateway', 'lego');
    fs.mkdirSync(sslDir, { recursive: true });
    fs.mkdirSync(path.join(legoDir, 'certificates'), { recursive: true });

    checkGatewayCertificate = checkGatewayCertificateFactory(homeDir);
  });

  afterEach(() => homeDir.remove());

  /**
   * @param {string} bundle
   * @param {string} [key]
   */
  function install(bundle, key) {
    if (bundle !== undefined) {
      fs.writeFileSync(path.join(sslDir, 'bundle.crt'), bundle, 'utf8');
    }

    if (key !== undefined) {
      fs.writeFileSync(path.join(sslDir, 'private.key'), key, { encoding: 'utf8', mode: 0o600 });
    }
  }

  /**
   * @param {Object} verdict
   * @return {string[]}
   */
  const codes = (list) => list.map(({ code }) => code);

  describe('leaf identification', () => {
    // The leaf is identified by matching key material rather than by position,
    // because that is the check that says which block belongs to private.key
    // and it works for every key type an authority might issue.
    it('should identify the leaf by its key material', () => {
      const { leaf, intermediate, root } = issueChain({ ip: EXTERNAL_IP });

      install(leaf.pem + intermediate.pem + root.pem, leaf.keyPem);

      const verdict = checkGatewayCertificate(config);

      expect(verdict.status).to.equal(CERTIFICATE_STATUS.CHECKS_PASSED);
      expect(verdict.installed.fingerprint256).to.be.a('string');
    });

    // An ordinary public chain contains a self-signed root. Only the block that
    // matches the key is self-sign tested, so carrying a root is not mistaken
    // for the certificate itself being self-signed.
    it('should accept a chain containing a self-signed root', () => {
      const { leaf, intermediate, root } = issueChain({ ip: EXTERNAL_IP });

      install(leaf.pem + intermediate.pem + root.pem, leaf.keyPem);

      const verdict = checkGatewayCertificate(config);

      expect(codes(verdict.reasons)).to.deep.equal([]);
      expect(verdict.status).to.equal(CERTIFICATE_STATUS.CHECKS_PASSED);
    });

    // Envoy reads the chain file in order and serves the first block as the
    // leaf. A bundle written the other way round is therefore broken at the
    // gateway however well its contents pair up, so finding the key's
    // certificate further down is a finding rather than a pass.
    it('should block on a bundle whose leaf is not the first block', () => {
      const { leaf, intermediate, root } = issueChain({ ip: EXTERNAL_IP });

      install(root.pem + intermediate.pem + leaf.pem, leaf.keyPem);

      const verdict = checkGatewayCertificate(config);

      expect(verdict.status).to.equal(CERTIFICATE_STATUS.INVALID);
      expect(codes(verdict.reasons)).to.include(CERTIFICATE_REASONS.BUNDLE_ORDER);
    });

    it('should name the position the key-matching certificate was found at', () => {
      const { leaf, intermediate, root } = issueChain({ ip: EXTERNAL_IP });

      install(root.pem + intermediate.pem + leaf.pem, leaf.keyPem);

      const [reason] = checkGatewayCertificate(config).reasons
        .filter(({ code }) => code === CERTIFICATE_REASONS.BUNDLE_ORDER);

      expect(reason.message).to.contain('3');
    });

    // A block whose END delimiter is missing is invisible to a match that pairs
    // BEGIN with END: the text is simply not matched, so a truncated bundle
    // looked identical to a well-formed one. Envoy refuses to load it, so
    // passing it here blesses a bundle the gateway cannot serve.
    it('should block on a bundle whose last certificate is unterminated', () => {
      const { leaf, intermediate } = issueChain({ ip: EXTERNAL_IP });
      const truncated = intermediate.pem.slice(0, intermediate.pem.length / 2);

      install(`${leaf.pem}${truncated}`, leaf.keyPem);

      const verdict = checkGatewayCertificate(config);

      expect(verdict.status).to.equal(CERTIFICATE_STATUS.INVALID);
      expect(codes(verdict.reasons)).to.include(CERTIFICATE_REASONS.BUNDLE_UNREADABLE);
    });

    it('should block on a bundle whose first certificate is unterminated', () => {
      const { leaf, intermediate } = issueChain({ ip: EXTERNAL_IP });
      const truncated = leaf.pem.slice(0, Math.floor(leaf.pem.length / 2));

      install(`${truncated}${leaf.pem}${intermediate.pem}`, leaf.keyPem);

      expect(codes(checkGatewayCertificate(config).reasons))
        .to.include(CERTIFICATE_REASONS.BUNDLE_UNREADABLE);
    });

    // The gateway loads a bundle carrying a stray END without complaint, so
    // rejecting it would refuse a node that works. Only an opening that never
    // closes means a block is actually missing.
    it('should accept a bundle carrying an END delimiter with no BEGIN', () => {
      const { leaf, intermediate } = issueChain({ ip: EXTERNAL_IP });

      install(`${leaf.pem}${intermediate.pem}-----END CERTIFICATE-----\n`, leaf.keyPem);

      const verdict = checkGatewayCertificate(config);

      expect(codes(verdict.reasons)).to.deep.equal([]);
      expect(verdict.status).to.equal(CERTIFICATE_STATUS.CHECKS_PASSED);
    });

    // A block Envoy will choke on is not something to pass over quietly. The
    // bundle is what the gateway loads, so an unparseable block in it is a
    // problem with the bundle whether or not a usable leaf sits beside it.
    it('should block on a bundle holding a certificate block that will not parse', () => {
      const { leaf, intermediate } = issueChain({ ip: EXTERNAL_IP });
      const corrupt = '-----BEGIN CERTIFICATE-----\nbm90IGEgY2VydGlmaWNhdGU=\n-----END CERTIFICATE-----\n';

      install(leaf.pem + corrupt + intermediate.pem, leaf.keyPem);

      const verdict = checkGatewayCertificate(config);

      expect(verdict.status).to.equal(CERTIFICATE_STATUS.INVALID);
      expect(codes(verdict.reasons)).to.include(CERTIFICATE_REASONS.BUNDLE_UNREADABLE);
    });

    // An operator's own self-signed certificate is usually marked as a CA.
    // Skipping CA blocks while looking for the leaf leaves nothing to judge,
    // and the node would be reported as having an unreadable bundle rather
    // than an untrusted one.
    it('should recognise a self-signed CA leaf as self-signed, not unreadable', () => {
      const selfSigned = issueCertificate({
        subject: { commonName: EXTERNAL_IP }, ip: EXTERNAL_IP, ca: true,
      });

      install(selfSigned.pem, selfSigned.keyPem);

      const verdict = checkGatewayCertificate(config);

      expect(codes(verdict.reasons)).to.include(CERTIFICATE_REASONS.SELF_SIGNED);
      expect(codes(verdict.reasons)).to.not.include(CERTIFICATE_REASONS.BUNDLE_UNREADABLE);
    });

    // lego passes --disable-cn for an IP certificate, so the leaf carries no
    // common name at all. Comparing subject to issuer would call that a match.
    it('should not call an empty-subject leaf self-signed', () => {
      const { leaf, intermediate } = issueChain({ ip: EXTERNAL_IP });

      install(leaf.pem + intermediate.pem, leaf.keyPem);

      const verdict = checkGatewayCertificate(config);

      expect(codes(verdict.reasons)).to.not.include(CERTIFICATE_REASONS.SELF_SIGNED);
      expect(verdict.installed.selfSigned).to.be.false();
    });

    // Comparing key material rather than running an RSA-only signature test is
    // what makes the check work for every key type a CA might issue.
    it('should pair an Ed25519 leaf with its key', () => {
      const { intermediate, root } = issueChain({ ip: EXTERNAL_IP });
      const leaf = issueEd25519Certificate(intermediate, { ip: EXTERNAL_IP });

      install(leaf.pem + intermediate.pem + root.pem, leaf.keyPem);

      const verdict = checkGatewayCertificate(config);

      expect(codes(verdict.reasons)).to.deep.equal([]);
      expect(verdict.status).to.equal(CERTIFICATE_STATUS.CHECKS_PASSED);
    });

    it('should report a key that matches no block in the bundle', () => {
      const { leaf, intermediate } = issueChain({ ip: EXTERNAL_IP });
      const other = issueCertificate({ ip: EXTERNAL_IP });

      install(leaf.pem + intermediate.pem, other.keyPem);

      expect(codes(checkGatewayCertificate(config).reasons))
        .to.deep.equal([CERTIFICATE_REASONS.KEY_MISMATCH]);
    });

    it('should report a missing bundle and a missing key separately', () => {
      const { leaf } = issueChain({ ip: EXTERNAL_IP });

      install(undefined, leaf.keyPem);
      expect(codes(checkGatewayCertificate(config).reasons))
        .to.deep.equal([CERTIFICATE_REASONS.BUNDLE_MISSING]);

      fs.rmSync(path.join(sslDir, 'private.key'));
      install(leaf.pem, undefined);
      expect(codes(checkGatewayCertificate(config).reasons))
        .to.deep.equal([CERTIFICATE_REASONS.KEY_MISSING]);
    });

    it('should report a bundle that holds no certificate', () => {
      const { leaf } = issueChain({ ip: EXTERNAL_IP });

      install('-----BEGIN CERTIFICATE-----\nnot base64 at all\n-----END CERTIFICATE-----\n', leaf.keyPem);

      expect(codes(checkGatewayCertificate(config).reasons))
        .to.deep.equal([CERTIFICATE_REASONS.BUNDLE_UNREADABLE]);
    });
  });

  describe('unusable private key', () => {
    // The gateway is handed the key file with no password or passphrase field
    // anywhere in its configuration, so a key dashmate cannot load is a key
    // Envoy cannot load either. Warning here would pass a node that serves no
    // TLS at all.
    it('should block on an encrypted key', () => {
      const certificate = issueCertificate({ ip: EXTERNAL_IP });

      install(certificate.pem, encryptPrivateKey(certificate.keys, 'passphrase'));

      const verdict = checkGatewayCertificate(config);

      expect(verdict.status).to.equal(CERTIFICATE_STATUS.INVALID);
      expect(codes(verdict.reasons)).to.deep.equal([CERTIFICATE_REASONS.KEY_UNUSABLE]);
    });

    // An operator whose problem is a permission bit must not be told their
    // certificate expired.
    it('should block on an unreadable key and say so without mentioning expiry', function it() {
      const certificate = issueCertificate({ ip: EXTERNAL_IP });

      install(certificate.pem, certificate.keyPem);

      const denied = Object.assign(new Error('permission denied'), { code: 'EACCES' });
      const readFileSync = this.sinon.stub(fs, 'readFileSync');
      readFileSync.callThrough();
      readFileSync.withArgs(path.join(sslDir, 'private.key')).throws(denied);

      const verdict = checkGatewayCertificate(config);

      expect(verdict.status).to.equal(CERTIFICATE_STATUS.INVALID);
      expect(codes(verdict.reasons)).to.deep.equal([CERTIFICATE_REASONS.KEY_UNUSABLE]);
      expect(verdict.reasons[0].message).to.contain('could not read');
      expect(verdict.reasons[0].message).to.contain(path.join(sslDir, 'private.key'));
      expect(verdict.reasons[0].message).to.not.contain('expired');
    });
  });

  describe('expiry', () => {
    it('should block on an expired leaf', () => {
      const { leaf, intermediate } = issueChain({ ip: EXTERNAL_IP, days: -3 });

      install(leaf.pem + intermediate.pem, leaf.keyPem);

      const verdict = checkGatewayCertificate(config);

      expect(verdict.status).to.equal(CERTIFICATE_STATUS.INVALID);
      expect(codes(verdict.reasons)).to.deep.equal([CERTIFICATE_REASONS.EXPIRED]);
      expect(verdict.expiresInDays).to.be.below(0);
    });

    // Renewal runs at two days remaining, so a two-day threshold would fire
    // through the whole window where renewal is routine and self-clearing.
    it('should warn only inside the last day', () => {
      const { leaf, intermediate } = issueChain({ ip: EXTERNAL_IP });
      install(leaf.pem + intermediate.pem, leaf.keyPem);

      expect(codes(checkGatewayCertificate(config).warnings))
        .to.not.include(CERTIFICATE_REASONS.EXPIRING_SOON);

      const soon = issueChain({ ip: EXTERNAL_IP, days: 0.5 });
      install(soon.leaf.pem + soon.intermediate.pem, soon.leaf.keyPem);

      const verdict = checkGatewayCertificate(config);
      expect(verdict.status).to.equal(CERTIFICATE_STATUS.WARN);
      expect(codes(verdict.warnings)).to.deep.equal([CERTIFICATE_REASONS.EXPIRING_SOON]);
    });
  });

  describe('identity', () => {
    // A current, key-matched certificate naming the wrong address is rejected
    // by every standards-compliant client, so passing it would report no
    // problem on a node that is dark to the network.
    it('should block on a leaf that names another address', () => {
      const { leaf, intermediate } = issueChain({ ip: '9.9.9.9' });

      install(leaf.pem + intermediate.pem, leaf.keyPem);

      const verdict = checkGatewayCertificate(config);

      expect(verdict.status).to.equal(CERTIFICATE_STATUS.INVALID);
      expect(codes(verdict.reasons)).to.deep.equal([CERTIFICATE_REASONS.IP_MISMATCH]);
    });

    it('should record the identity check as skipped when no address is configured', () => {
      const { leaf, intermediate } = issueChain({ ip: '9.9.9.9' });

      config.set('externalIp', null);
      install(leaf.pem + intermediate.pem, leaf.keyPem);

      const verdict = checkGatewayCertificate(config);

      expect(verdict.skipped).to.deep.equal(['IDENTITY']);
      expect(codes(verdict.reasons)).to.not.include(CERTIFICATE_REASONS.IP_MISMATCH);
    });

    // Node's own tls.checkServerIdentity does not consult the common name for
    // an IP identifier, and neither do browsers. A certificate that carries the
    // address only in its subject is rejected by every client that matters, so
    // passing it here would report no problem on a node nothing can connect to.
    it('should block on a leaf that carries the address only in its common name', () => {
      const certificate = issueCertificate({ subject: { commonName: EXTERNAL_IP } });

      install(certificate.pem, certificate.keyPem);

      expect(codes(checkGatewayCertificate(config).reasons))
        .to.include(CERTIFICATE_REASONS.IP_MISMATCH);
    });

    it('should say the address is missing from the SAN rather than wrong', () => {
      const certificate = issueCertificate({ subject: { commonName: EXTERNAL_IP } });

      install(certificate.pem, certificate.keyPem);

      const [reason] = checkGatewayCertificate(config).reasons
        .filter(({ code }) => code === CERTIFICATE_REASONS.IP_MISMATCH);

      expect(reason.message).to.contain('subject alternative name');
    });
  });

  // A certificate whose validity has not started yet is unservable in exactly
  // the way an expired one is - clients reject it on the same field. This is a
  // plain validity condition and infers nothing about the clock: a fast local
  // clock and a genuinely future notBefore are indistinguishable from here, so
  // no conclusion is drawn about which one it is.
  describe('validity start', () => {
    it('should block on a certificate that is not valid yet', () => {
      const certificate = issueCertificate({ ip: EXTERNAL_IP, startsInDays: 5 });

      install(certificate.pem, certificate.keyPem);

      const verdict = checkGatewayCertificate(config);

      expect(verdict.status).to.equal(CERTIFICATE_STATUS.INVALID);
      expect(codes(verdict.reasons)).to.include(CERTIFICATE_REASONS.NOT_YET_VALID);
    });

    it('should not report a certificate already in its validity window', () => {
      const certificate = issueCertificate({ ip: EXTERNAL_IP, startsInDays: -1 });

      install(certificate.pem, certificate.keyPem);

      expect(codes(checkGatewayCertificate(config).reasons))
        .to.not.include(CERTIFICATE_REASONS.NOT_YET_VALID);
    });
  });

  describe('provider agreement', () => {
    /**
     * @param {Object} pair
     */
    function installAsLegoPair(pair) {
      install(pair.pem, pair.keyPem);
      fs.writeFileSync(path.join(legoDir, 'certificates', `${EXTERNAL_IP}.crt`), pair.pem);
      fs.writeFileSync(path.join(legoDir, 'certificates', `${EXTERNAL_IP}.key`), pair.keyPem);
    }

    // A kill between installing the pair and writing the provider leaves this
    // behind. Warning about it lets the helper keep renewing the old provider
    // while the installed six-day certificate runs out, so it never repairs
    // itself - blocking is what makes the next run converge.
    it('should block when the installed pair is the lego pair but the provider is not', () => {
      const { leaf, intermediate } = issueChain({ ip: EXTERNAL_IP });

      config.set('platform.gateway.ssl.provider', 'zerossl');
      installAsLegoPair({ pem: leaf.pem + intermediate.pem, keyPem: leaf.keyPem });

      const verdict = checkGatewayCertificate(config);

      expect(verdict.status).to.equal(CERTIFICATE_STATUS.INVALID);
      expect(codes(verdict.reasons)).to.deep.equal([CERTIFICATE_REASONS.SWITCH_INCOMPLETE]);
    });

    it('should only warn when the issuer disagrees and the pair is not the lego pair', () => {
      const { leaf, intermediate } = issueChain({ ip: EXTERNAL_IP });

      config.set('platform.gateway.ssl.provider', 'zerossl');
      install(leaf.pem + intermediate.pem, leaf.keyPem);

      const verdict = checkGatewayCertificate(config);

      expect(verdict.status).to.equal(CERTIFICATE_STATUS.WARN);
      expect(codes(verdict.warnings)).to.deep.equal([CERTIFICATE_REASONS.PROVIDER_MISMATCH]);
    });

    it('should not judge the issuer of a certificate the operator supplied', () => {
      const { leaf, intermediate } = issueChain({ ip: EXTERNAL_IP });

      config.set('platform.gateway.ssl.provider', 'file');
      install(leaf.pem + intermediate.pem, leaf.keyPem);

      expect(codes(checkGatewayCertificate(config).warnings))
        .to.not.include(CERTIFICATE_REASONS.PROVIDER_MISMATCH);
    });
  });

  describe('self-signed enforcement', () => {
    // Dashmate's own setup wizard offers self-signed to a mainnet evolution
    // fullnode, so blocking it unconditionally would break update for a
    // configuration dashmate created.
    it('should warn rather than block on a node that is not a masternode', () => {
      const certificate = issueCertificate({ ip: EXTERNAL_IP });

      config.set('core.masternode.enable', false);
      config.set('platform.gateway.ssl.provider', 'self-signed');
      install(certificate.pem, certificate.keyPem);

      const verdict = checkGatewayCertificate(config);

      expect(verdict.status).to.equal(CERTIFICATE_STATUS.WARN);
      expect(codes(verdict.warnings)).to.deep.equal([CERTIFICATE_REASONS.SELF_SIGNED]);
      expect(verdict.warnings[0].message).to.contain('not publicly trusted');
    });

    it('should block on a registered masternode', () => {
      const certificate = issueCertificate({ ip: EXTERNAL_IP });

      config.set('platform.gateway.ssl.provider', 'self-signed');
      install(certificate.pem, certificate.keyPem);

      const verdict = checkGatewayCertificate(config);

      expect(verdict.status).to.equal(CERTIFICATE_STATUS.INVALID);
      expect(codes(verdict.reasons)).to.deep.equal([CERTIFICATE_REASONS.SELF_SIGNED]);
    });
  });

  describe('unmanaged SSL', () => {
    // The flag appears in no template - the gateway terminates TLS from the
    // bundle unconditionally - so on its own it means "dashmate is not managing
    // renewal", not "this node serves plaintext".
    it('should warn when the checks pass but dashmate is not managing renewal', () => {
      const { leaf, intermediate } = issueChain({ ip: EXTERNAL_IP });

      config.set('platform.gateway.ssl.enabled', false);
      install(leaf.pem + intermediate.pem, leaf.keyPem);

      const verdict = checkGatewayCertificate(config);

      expect(verdict.status).to.equal(CERTIFICATE_STATUS.WARN);
      expect(codes(verdict.warnings)).to.deep.equal([CERTIFICATE_REASONS.SSL_UNMANAGED]);
    });

    it('should block when nothing is managing renewal and the certificate is broken', () => {
      const { leaf, intermediate } = issueChain({ ip: EXTERNAL_IP, days: -3 });

      config.set('platform.gateway.ssl.enabled', false);
      install(leaf.pem + intermediate.pem, leaf.keyPem);

      const verdict = checkGatewayCertificate(config);

      expect(verdict.status).to.equal(CERTIFICATE_STATUS.INVALID);
      expect(codes(verdict.reasons)).to.deep.equal([
        CERTIFICATE_REASONS.EXPIRED,
        CERTIFICATE_REASONS.SSL_DISABLED,
      ]);
    });
  });

  // The check reads local files. It does not validate the chain to a public
  // root, does not check revocation and never opens a connection, so nothing
  // it returns may be called valid, trusted, usable or reachable.
  it('should never report a certificate as valid', () => {
    const { leaf, intermediate } = issueChain({ ip: EXTERNAL_IP });

    install(leaf.pem + intermediate.pem, leaf.keyPem);

    const verdict = checkGatewayCertificate(config);

    expect(verdict.status).to.equal('CHECKS_PASSED');
    expect(Object.values(CERTIFICATE_STATUS)).to.not.include('VALID');
    expect(verdict.reasons).to.be.an('array');
    expect(verdict.warnings).to.be.an('array');
    expect(verdict.skipped).to.be.an('array');
    expect(verdict.provider).to.equal('letsencrypt');
  });
});
