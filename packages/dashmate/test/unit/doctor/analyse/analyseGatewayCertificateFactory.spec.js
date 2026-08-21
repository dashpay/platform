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
    expect(problems[0].getSolution()).to.include('dashmate restart --config base --platform');
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

  describe('the certificate on disk', () => {
    /**
     * @param {Object} installed
     * @param {Object} [servedCertificate]
     * @return {Problem[]}
     */
    function analyseInstalled(installed, servedCertificate) {
      samples.setServiceInfo('gateway', 'installedCertificate', installed);

      if (servedCertificate) {
        samples.setServiceInfo('gateway', 'servedCertificate', servedCertificate);
      }

      return analyseGatewayCertificate(samples);
    }

    // Under the documented upgrade procedure the node is stopped when the
    // certificate check fails, and a stopped gateway answers no TLS connection
    // - so the probe records nothing and every problem on disk went unreported.
    // That is exactly the node an operator has just been told to run doctor on.
    it('should report a problem for a stopped node with a broken bundle', () => {
      const problems = analyseInstalled({
        status: 'INVALID',
        reasons: [{ code: 'EXPIRED', message: 'The installed certificate expired on 2026-05-01' }],
        warnings: [],
      });

      expect(problems).to.have.lengthOf(1);
      expect(problems[0].getSeverity()).to.equal(SEVERITY.HIGH);
      expect(problems[0].getDescription()).to.include('expired on 2026-05-01');
    });

    // An operator who reads this is deciding whether to stop updating. Images
    // keep arriving whatever the certificate does, and saying so is what keeps
    // a client-reachability problem from being read as a software-delivery one.
    it('should say that updates still deliver images', () => {
      const [problem] = analyseInstalled({
        status: 'INVALID',
        reasons: [{ code: 'EXPIRED', message: 'expired' }],
        warnings: [],
      });

      expect(problem.getSolution()).to.include('still pulls new images');
      expect(problem.getSolution()).to.include('exits non-zero');
    });

    // Doctor is run against a named node, and a solution pasted without one
    // targets whichever config happens to be the default. For `ssl obtain`
    // that re-issues a certificate for a different node's address and rewrites
    // that node's provider - a mutation of the wrong machine.
    it('should put the node it analysed on every command it suggests', () => {
      const problems = analyseInstalled({
        status: 'INVALID',
        reasons: [{ code: 'EXPIRED', message: 'expired' }],
        warnings: [{ code: 'EXPIRING_SOON', message: 'expires tomorrow' }],
      });

      expect(problems).to.have.lengthOf(2);

      problems.forEach((problem) => {
        const commands = problem.getSolution()
          .split('\n')
          .map((line) => line.trim())
          .filter((line) => line.startsWith('dashmate '));

        expect(commands).to.have.length.greaterThan(0);
        commands.forEach((command) => {
          expect(command, command).to.contain(`--config ${config.getName()}`);
        });
      });
    });

    // Telling someone who is reading a doctor report to run doctor is circular.
    it('should not suggest running doctor as the fix for a doctor problem', () => {
      const [problem] = analyseInstalled({
        status: 'INVALID',
        reasons: [{ code: 'EXPIRED', message: 'expired' }],
        warnings: [],
      });

      expect(problem.getSolution()).to.not.match(/^\s*dashmate doctor\b/m);
    });

    it('should report each warning separately and more quietly', () => {
      const problems = analyseInstalled({
        status: 'WARN',
        reasons: [],
        warnings: [
          { code: 'EXPIRING_SOON', message: 'expires tomorrow' },
          { code: 'PROVIDER_MISMATCH', message: 'issuer disagrees' },
        ],
      });

      expect(problems).to.have.lengthOf(2);
      problems.forEach((problem) => expect(problem.getSeverity()).to.equal(SEVERITY.LOW));
    });

    it('should say nothing when the checks passed', () => {
      expect(analyseInstalled({ status: 'CHECKS_PASSED', reasons: [], warnings: [] }))
        .to.have.lengthOf(0);
    });

    it('should still analyse what the gateway serves', () => {
      const problems = analyseInstalled(
        { status: 'CHECKS_PASSED', reasons: [], warnings: [] },
        served({ certificate: { fingerprint256: 'AA:BB', validTo: validTo(-1) } }),
      );

      expect(problems).to.have.lengthOf(1);
      expect(problems[0].getDescription()).to.include('expired');
    });
  });
});
