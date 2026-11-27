import fs from 'fs';
import path from 'path';
import findCertificatePairProblem from '../../../src/ssl/findCertificatePairProblem.js';
import checkGatewayCertificateFactory, {
  CERTIFICATE_REASONS,
} from '../../../src/ssl/checkGatewayCertificateFactory.js';
import HomeDir from '../../../src/config/HomeDir.js';
import getBaseConfigFactory from '../../../configs/defaults/getBaseConfigFactory.js';
import {
  issueCertificate,
  issueCertificateWithUnreadableExpiry,
  issueChain,
} from '../../../src/test/certificateFixtures.js';

describe('findCertificatePairProblem', () => {
  let chain;
  let selfSigned;

  before(function before() {
    // RSA key generation is slow in pure JS.
    this.timeout(60000);
    chain = issueChain({ days: 30 });
    selfSigned = issueCertificate({ ip: '1.2.3.4' });
  });

  it('should accept a key\'s certificate followed by the authority that issued it', () => {
    const bundle = `${chain.leaf.pem}\n${chain.intermediate.pem}`;

    expect(findCertificatePairProblem(bundle, chain.leaf.keyPem)).to.be.null();
  });

  it('should accept a self-signed certificate on its own', () => {
    expect(findCertificatePairProblem(selfSigned.pem, selfSigned.keyPem)).to.be.null();
  });

  // ZeroSSL writes the key's certificate and then its authority's into one
  // file. A write a full disk cuts short right after the first block still
  // parses, but clients cannot build a chain from it.
  it('should refuse a bundle that ends after the key\'s certificate', () => {
    expect(findCertificatePairProblem(`${chain.leaf.pem}\n`, chain.leaf.keyPem))
      .to.include('issued');
  });

  it('should refuse a bundle cut inside a block', () => {
    const bundle = `${chain.leaf.pem}\n${chain.intermediate.pem}`;

    expect(findCertificatePairProblem(bundle.slice(0, -200), chain.leaf.keyPem)).to.be.a('string');
  });

  it('should refuse an expired certificate', function it() {
    this.timeout(30000);
    const expired = issueCertificate({ ip: '1.2.3.4', days: -1 });

    expect(findCertificatePairProblem(expired.pem, expired.keyPem)).to.include('expired');
  });

  // Signed over the date it carries, so only the date check can refuse it:
  // an invalid Date compares as neither before nor after now.
  it('should refuse a certificate whose expiry cannot be read', () => {
    const leaf = issueCertificateWithUnreadableExpiry(chain.intermediate);
    const bundle = `${leaf.pem}\n${chain.intermediate.pem}`;

    expect(findCertificatePairProblem(bundle, leaf.keyPem))
      .to.equal('its certificate\'s validity dates cannot be read');
  });

  // ssl obtain checks the pair with this before it enables TLS, and update's
  // certificate check refuses the same certificate afterwards: clients reject
  // it on the same field as an expired one.
  it('should refuse a certificate that is not valid yet, as update\'s check does', function it() {
    this.timeout(30000);
    const future = issueCertificate({
      subject: {}, issuer: chain.intermediate, ip: '1.2.3.4', startsInDays: 2,
    });
    const bundle = `${future.pem}\n${chain.intermediate.pem}`;

    expect(findCertificatePairProblem(bundle, future.keyPem)).to.include('not valid until');

    const homeDir = HomeDir.createTemp();

    try {
      const config = getBaseConfigFactory(homeDir)();
      config.set('externalIp', '1.2.3.4');
      const sslDir = homeDir.joinPath(config.getName(), 'platform', 'gateway', 'ssl');
      fs.mkdirSync(sslDir, { recursive: true });
      fs.writeFileSync(path.join(sslDir, 'bundle.crt'), bundle);
      fs.writeFileSync(path.join(sslDir, 'private.key'), future.keyPem);

      const verdict = checkGatewayCertificateFactory(homeDir)(config);

      expect(verdict.reasons.map(({ code }) => code))
        .to.include(CERTIFICATE_REASONS.NOT_YET_VALID);
    } finally {
      homeDir.remove();
    }
  });

  // Equal names are a convention, not a signature: any authority can issue a
  // certificate whose issuer name repeats its subject.
  it('should not take a certificate that only names itself as its issuer for self-signed', function it() {
    this.timeout(30000);
    const namedLikeItself = issueCertificate({
      subject: { commonName: '1.2.3.4' },
      issuer: { subject: { commonName: '1.2.3.4' }, keys: chain.intermediate.keys },
      ip: '1.2.3.4',
    });

    expect(findCertificatePairProblem(namedLikeItself.pem, namedLikeItself.keyPem))
      .to.include('issued');
  });
});
