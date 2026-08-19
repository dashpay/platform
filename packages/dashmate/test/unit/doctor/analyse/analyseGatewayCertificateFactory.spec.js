import getBaseConfigFactory from '../../../../configs/defaults/getBaseConfigFactory.js';
import analyseGatewayCertificateFactory from '../../../../src/doctor/analyse/analyseGatewayCertificateFactory.js';
import { SEVERITY } from '../../../../src/doctor/Prescription.js';
import Samples from '../../../../src/doctor/Samples.js';

const EXTERNAL_IP = '198.51.100.7';

/**
 * @param {number} days - relative to now, negative for an expired certificate
 * @return {string}
 */
function validTo(days) {
  return new Date(Date.now() + days * 24 * 60 * 60 * 1000).toUTCString();
}

describe('analyseGatewayCertificateFactory', () => {
  let analyseGatewayCertificate;
  let config;
  let samples;

  /**
   * @param {Object} servedCertificate
   * @return {Problem[]}
   */
  function analyse(servedCertificate) {
    samples.setServiceInfo('gateway', 'servedCertificate', servedCertificate);

    return analyseGatewayCertificate(samples);
  }

  /**
   * @param {Object} overrides
   * @return {Object}
   */
  function served(overrides = {}) {
    return {
      state: 'served',
      port: 443,
      certificate: { fingerprint256: 'AA:BB', validTo: validTo(30) },
      chainVerified: true,
      chainError: null,
      identityVerified: true,
      identityError: null,
      matchesOnDisk: true,
      ...overrides,
    };
  }

  beforeEach(() => {
    config = getBaseConfigFactory()();

    config.set('platform.enable', true);
    config.set('externalIp', EXTERNAL_IP);

    samples = new Samples();
    samples.setDashmateConfig(config);

    analyseGatewayCertificate = analyseGatewayCertificateFactory();
  });

  it('should report no problem for a healthy certificate', () => {
    expect(analyse(served())).to.be.empty();
  });

  it('should report an expired certificate that clients cannot connect to', () => {
    const problems = analyse(served({
      certificate: { fingerprint256: 'AA:BB', validTo: validTo(-158) },
    }));

    expect(problems).to.have.lengthOf(1);
    expect(problems[0].getDescription()).to.include('expired');
    expect(problems[0].getSeverity()).to.equal(SEVERITY.HIGH);
    expect(problems[0].getSolution()).to.include('dashmate_helper');
  });

  it('should distinguish a certificate that was renewed but never reached the gateway', () => {
    const problems = analyse(served({
      certificate: { fingerprint256: 'AA:BB', validTo: validTo(-2) },
      matchesOnDisk: false,
    }));

    expect(problems).to.have.lengthOf(1);
    expect(problems[0].getDescription()).to.include('newer one is already present on disk');
    expect(problems[0].getSolution()).to.include('dashmate restart');
  });

  it('should warn before the outage when a renewed certificate has not been picked up', () => {
    const problems = analyse(served({ matchesOnDisk: false }));

    expect(problems).to.have.lengthOf(1);
    expect(problems[0].getDescription()).to.include('older certificate than the one on disk');
    expect(problems[0].getSeverity()).to.equal(SEVERITY.HIGH);
  });

  it('should report an untrusted certificate separately from expiry', () => {
    const problems = analyse(served({
      chainVerified: false,
      chainError: 'UNABLE_TO_VERIFY_LEAF_SIGNATURE',
    }));

    expect(problems).to.have.lengthOf(1);
    expect(problems[0].getDescription()).to.include('not trusted by standard clients');
  });

  it('should treat an identity mismatch as not having reached this node and stop there', () => {
    // A different node or a proxy answering on the port returns a certificate that says nothing
    // about this node, so reporting it as stale or expired would send the operator the wrong way.
    const problems = analyse(served({
      identityVerified: false,
      identityError: 'Host: 198.51.100.7 is not in the cert\'s altnames',
      matchesOnDisk: false,
      certificate: { fingerprint256: 'CC:DD', validTo: validTo(-10) },
    }));

    expect(problems).to.have.lengthOf(1);
    expect(problems[0].getDescription()).to.include('not valid for 198.51.100.7');
  });

  it('should judge expiry against the time the samples were taken, not the time of analysis', () => {
    // Reports are commonly opened days after collection, and a Let's Encrypt certificate for an
    // IP address lives about six days, so judging at analysis time reports healthy nodes as dead.
    samples.date = new Date(Date.now() - 10 * 24 * 60 * 60 * 1000);

    const problems = analyse(served({
      certificate: { fingerprint256: 'AA:BB', validTo: validTo(-4) },
    }));

    expect(problems).to.be.empty();
  });

  it('should report a closed port 80 as a likely cause when a certificate problem exists', () => {
    samples.setServiceInfo('gateway', 'validationHttpPort', 'CLOSED');

    const problems = analyse(served({
      certificate: { fingerprint256: 'AA:BB', validTo: validTo(-3) },
    }));

    expect(problems).to.have.lengthOf(2);
    expect(problems[1].getDescription()).to.include('port 80');
  });

  it('should not report a closed port 80 on a node whose certificate is healthy', () => {
    // The port is only bound for the seconds a validation takes, so an external check finds it
    // closed on actively renewing nodes too. Alone it would fire far more often than it is right.
    samples.setServiceInfo('gateway', 'validationHttpPort', 'CLOSED');

    expect(analyse(served())).to.be.empty();
  });

  it('should report a gateway that does not answer TLS', () => {
    const problems = analyse({ state: 'unreachable', reason: 'ECONNREFUSED' });

    expect(problems).to.have.lengthOf(1);
    expect(problems[0].getDescription()).to.include('did not answer');
    expect(problems[0].getSeverity()).to.equal(SEVERITY.MEDIUM);
  });

  it('should report nothing when the probe was skipped', () => {
    expect(analyse({ state: 'skipped', reason: 'self-signed' })).to.be.empty();
  });

  it('should report nothing when platform is disabled', () => {
    config.set('platform.enable', false);

    expect(analyse(served({ certificate: { validTo: validTo(-100) } }))).to.be.empty();
  });
});
