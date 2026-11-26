import findCertificatePairProblem from '../../../src/ssl/findCertificatePairProblem.js';
import { issueCertificate, issueChain } from '../../../src/test/certificateFixtures.js';

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
});
