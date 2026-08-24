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
   * Record that the files on disk were judged sound, for the exact pair the
   * wire probe sampled.
   *
   * @param {string} fingerprint256
   */
  function installedIsUsable(fingerprint256) {
    samples.setServiceInfo('gateway', 'installedCertificate', {
      status: 'CHECKS_PASSED',
      reasons: [],
      warnings: [],
      fingerprint256,
    });
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

  it('should report nothing on a network where update does not enforce', () => {
    // A local node uses a self-signed certificate by design. Reporting it as a
    // problem tells an operator their healthy node is broken, and prescribes a
    // publicly-issued certificate for an address no authority can reach.
    config.set('network', 'local');
    config.set('platform.gateway.ssl.provider', 'self-signed');

    samples.setServiceInfo('gateway', 'installedCertificate', {
      status: 'INVALID',
      reasons: [{ code: 'SELF_SIGNED', message: 'The certificate is self-signed.' }],
      warnings: [],
      fingerprint256: 'AA:BB',
    });

    expect(analyseGatewayCertificate(samples)).to.be.empty();
  });

  it('should replace rather than reinstall a certificate issued for another address', () => {
    // Reinstalling cannot fix an address the certificate does not carry, and
    // the reuse check is weaker than this one, so an unforced command can hand
    // back the same rejected certificate.
    samples.setServiceInfo('gateway', 'installedCertificate', {
      status: 'INVALID',
      reasons: [{ code: 'IP_MISMATCH', message: 'The certificate is not valid for this address.' }],
      warnings: [],
      fingerprint256: 'AA:BB',
    });

    const [problem] = analyseGatewayCertificate(samples);

    expect(problem.getSolution()).to.include('--force');
  });

  // Nothing can be issued for an address dashmate does not have, and the
  // obtain command refuses to start without one, so advising it here sends the
  // operator to a command that fails before it does anything.
  it('should prescribe the address rather than an obtain that cannot run', () => {
    samples.setServiceInfo('gateway', 'installedCertificate', {
      status: 'INVALID',
      reasons: [{
        code: 'NO_EXTERNAL_IP',
        message: "This node's public address is not set",
      }],
      warnings: [],
      fingerprint256: 'AA:BB',
    });

    const [problem] = analyseGatewayCertificate(samples);

    const solution = problem.getSolution();

    // Obtain still belongs here - it is what the operator runs once an address
    // exists - but only after the setting that makes it able to run at all.
    expect(solution).to.include('externalIp');
    expect(solution.indexOf('externalIp')).to.be.lessThan(solution.indexOf('ssl obtain'));
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
    // Renewed means the disk copy outlives the served one, and that it was
    // judged sound. Leaving either out let the branch claim a direction and a
    // safety it had never checked.
    installedIsUsable('CC:DD');

    const problems = analyse(served({
      certificate: { fingerprint256: 'AA:BB', validTo: validTo(-2) },
      matchesOnDisk: false,
      onDisk: { fingerprint256: 'CC:DD', validTo: validTo(30) },
    }));

    expect(problems).to.have.lengthOf(1);
    expect(problems[0].getDescription()).to.include('newer one has already been saved and is ready to use');
    expect(problems[0].getSolution()).to.include('dashmate restart --config base --platform');
  });

  it('should warn before the outage when a renewed certificate has not been picked up', () => {
    // Renewed means the disk copy outlives the served one and was judged
    // sound; together that is what makes a restart the right advice here.
    installedIsUsable('CC:DD');

    const problems = analyse(served({
      matchesOnDisk: false,
      onDisk: { fingerprint256: 'CC:DD', validTo: validTo(60) },
    }));

    expect(problems).to.have.lengthOf(1);
    expect(problems[0].getDescription()).to.include('older certificate than the one that has been saved');
    expect(problems[0].getSeverity()).to.equal(SEVERITY.HIGH);
  });

  it('should report an untrusted certificate separately from expiry', () => {
    const problems = analyse(served({
      chainVerified: false,
      chainError: 'UNABLE_TO_VERIFY_LEAF_SIGNATURE',
    }));

    expect(problems).to.have.lengthOf(1);
    expect(problems[0].getDescription()).to.include('not trusted by ordinary clients');

    // The raw verification code is what an operator cannot read, so it is
    // translated rather than printed.
    expect(problems[0].getDescription()).to.not.include('UNABLE_TO_VERIFY_LEAF_SIGNATURE');
    expect(problems[0].getDescription()).to.include('only one certificate was sent');
  });

  // This code is returned for a complete, correct bundle whose root the machine
  // does not trust, as well as for one that really is missing certificates.
  // Telling the first operator their bundle is incomplete sends them to repair
  // something that is not broken.
  it('should not claim certificates are missing when the issuer is merely untrusted', () => {
    const problems = analyse(served({
      chainVerified: false,
      chainError: 'UNABLE_TO_GET_ISSUER_CERT_LOCALLY',
    }));

    const description = problems[0].getDescription();

    expect(description).to.not.match(/were not sent/);
    expect(description).to.contain('does not trust');
  });

  // Only one certificate arrived, which is established rather than guessed -
  // but why its issuer could not be found is not, so both readings are named.
  it('should name both readings when only the certificate itself was sent', () => {
    const problems = analyse(served({
      chainVerified: false,
      chainError: 'UNABLE_TO_VERIFY_LEAF_SIGNATURE',
    }));

    expect(problems[0].getDescription()).to.contain('only one certificate');
  });

  // An unrecognised code is still searchable; a vague paraphrase of it is not.
  it('should pass through a verification code it has no wording for', () => {
    const problems = analyse(served({ chainVerified: false, chainError: 'SOME_NEW_CODE' }));

    expect(problems[0].getDescription()).to.include('SOME_NEW_CODE');
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
    expect(problems[0].getDescription()).to.include('not issued for this node\'s address');
    expect(problems[0].getDescription()).to.include('198.51.100.7');

    // "altnames" is the certificate's own vocabulary, not the operator's.
    expect(problems[0].getDescription()).to.not.include('altnames');
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

  // The branch fires on the two certificates merely differing. Asserting a
  // direction without comparing them, and then advising a restart on that
  // basis, swaps a valid served certificate for an expired one and takes a
  // working node dark - which is the outcome this whole feature exists to
  // prevent, produced by its own remediation.
  describe('when the served and on-disk certificates differ', () => {
    it('should advise a restart only when the disk copy is the newer one', () => {
      installedIsUsable('CC:DD');

      const problems = analyse(served({
        matchesOnDisk: false,
        onDisk: { fingerprint256: 'CC:DD', validTo: validTo(60) },
      }));

      const [problem] = problems.filter((p) => p.getDescription().includes('has been saved'));

      expect(problem.getDescription()).to.include('older certificate than the one that has been saved');
      expect(problem.getSolution()).to.include('dashmate restart');
    });

    it('should not advise a restart when the disk copy is the stale one', () => {
      const problems = analyse(served({
        matchesOnDisk: false,
        onDisk: { fingerprint256: 'CC:DD', validTo: validTo(-158) },
      }));

      const [problem] = problems.filter((p) => p.getDescription().includes('has been saved'));

      expect(problem.getDescription()).to.not.include('older certificate than the one that has been saved');
      expect(problem.getSolution()).to.not.match(/dashmate restart/);
    });

    it('should claim no direction when it cannot compare them', () => {
      const problems = analyse(served({ matchesOnDisk: false, onDisk: null }));

      const [problem] = problems.filter((p) => p.getDescription().includes('has been saved'));

      expect(problem.getDescription()).to.not.include('older certificate than the one that has been saved');
      expect(problem.getSolution()).to.not.match(/dashmate restart/);
    });
  });

  // The expired-served branch makes the same unchecked claim: it calls any
  // differing disk copy newer and advises a restart, so a node serving an
  // expired certificate with an equally dead one on disk is told a restart
  // will fix it.
  describe('when the served certificate has expired and the disk copy differs', () => {
    it('should advise a restart only when the disk copy really is newer', () => {
      installedIsUsable('CC:DD');

      const [problem] = analyse(served({
        certificate: { fingerprint256: 'AA:BB', validTo: validTo(-1) },
        matchesOnDisk: false,
        onDisk: { fingerprint256: 'CC:DD', validTo: validTo(30) },
      }));

      expect(problem.getDescription()).to.include('newer one has already been saved and is ready to use');
      expect(problem.getSolution()).to.include('dashmate restart');
    });

    it('should not advise a restart when the disk copy is no better', () => {
      const [problem] = analyse(served({
        certificate: { fingerprint256: 'AA:BB', validTo: validTo(-1) },
        matchesOnDisk: false,
        onDisk: { fingerprint256: 'CC:DD', validTo: validTo(-158) },
      }));

      expect(problem.getDescription()).to.not.include('newer one has already been saved and is ready to use');
      expect(problem.getSolution()).to.not.match(/dashmate restart/);
      expect(problem.getSolution()).to.include('dashmate ssl obtain');
    });

    it('should not advise a restart when there is nothing to compare', () => {
      const [problem] = analyse(served({
        certificate: { fingerprint256: 'AA:BB', validTo: validTo(-1) },
        matchesOnDisk: false,
        onDisk: null,
      }));

      expect(problem.getDescription()).to.not.include('newer one has already been saved and is ready to use');
      expect(problem.getSolution()).to.not.match(/dashmate restart/);
    });
  });

  // A verdict that is absent, merely not-invalid, or about a different pair is
  // not evidence that the file is safe to load. A report collected by an older
  // dashmate carries no verdict at all, and a renewal landing between the two
  // samples means the pair judged is not the pair measured.
  describe('when the disk copy cannot be shown to be usable', () => {
    it('should not advise a restart when nothing judged the pair', () => {
      const problems = analyse(served({
        matchesOnDisk: false,
        onDisk: { fingerprint256: 'CC:DD', validTo: validTo(60) },
      }));

      problems.forEach((problem) => {
        expect(problem.getSolution()).to.not.match(/dashmate restart/);
      });
    });

    it('should not advise a restart when the verdict only fell short of failing', () => {
      samples.setServiceInfo('gateway', 'installedCertificate', {
        status: 'WARN',
        reasons: [],
        warnings: [{ code: 'PROVIDER_MISMATCH', message: 'issuer disagrees' }],
        fingerprint256: 'CC:DD',
      });

      const problems = analyse(served({
        matchesOnDisk: false,
        onDisk: { fingerprint256: 'CC:DD', validTo: validTo(60) },
      }));

      problems.forEach((problem) => {
        expect(problem.getSolution()).to.not.match(/dashmate restart/);
      });
    });

    it('should not advise a restart when the verdict judged a different pair', () => {
      installedIsUsable('EE:FF');

      const problems = analyse(served({
        matchesOnDisk: false,
        onDisk: { fingerprint256: 'CC:DD', validTo: validTo(60) },
      }));

      problems.forEach((problem) => {
        expect(problem.getSolution()).to.not.match(/dashmate restart/);
      });
    });
  });

  // The disk copy can fail the usability gate while genuinely being the newer
  // of the two, so saying it is not newer would be a second wrong claim in
  // place of the one that was removed.
  it('should not deny the disk copy is newer when the objection is something else', () => {
    installedIsUsable('EE:FF');

    const [problem] = analyse(served({
      matchesOnDisk: false,
      onDisk: { fingerprint256: 'CC:DD', validTo: validTo(60) },
    }));

    expect(problem.getDescription()).to.not.match(/is no newer|not the newer/i);
    expect(problem.getSolution()).to.not.match(/dashmate restart/);
  });

  // This branch means the connection did not reach this node's gateway at all -
  // another config, a proxy, or a second node is answering on that port. So
  // reissuing the certificate and restarting Platform fixes nothing: the port
  // is still taken, and the operator has bought an outage for it.
  describe('when the connection did not reach this node', () => {
    const hijacked = () => served({
      identityVerified: false,
      identityError: 'Host: 198.51.100.7. is not in the cert\'s altnames',
    });

    it('should send the operator to find what is answering, not to restart', () => {
      const [problem] = analyse(hijacked());

      expect(problem.getSolution()).to.not.match(/dashmate restart/);
      expect(problem.getSolution()).to.contain('answering on port 443');
    });

    // Reissuing is only the remedy once the gateway is known to be the thing
    // answering, so it is offered on that condition rather than as the step.
    it('should offer reissuing only once the gateway is known to be answering', () => {
      const [problem] = analyse(hijacked());

      expect(problem.getSolution()).to.contain('If this node\'s gateway is answering');
      expect(problem.getSolution()).to.contain('dashmate ssl obtain --config base --force');
    });
  });

  // A later expiry says nothing about whether the disk pair can be served. The
  // sample carries only a fingerprint and a date; key pairing, address and
  // self-signature come from the installed verdict, which is collected in the
  // same run. Loading a later-expiring but unusable pair over a working one is
  // the same outage the date comparison was added to prevent.
  describe('when the disk copy is newer but not usable', () => {
    [
      ['the pair does not match its key', 'KEY_MISMATCH'],
      ['it names another address', 'IP_MISMATCH'],
    ].forEach(([name, code]) => {
      it(`should not advise a restart when ${name}`, () => {
        samples.setServiceInfo('gateway', 'installedCertificate', {
          status: 'INVALID',
          reasons: [{ code, message: 'the installed pair is unusable' }],
          warnings: [],
        });

        const problems = analyse(served({
          matchesOnDisk: false,
          onDisk: { fingerprint256: 'CC:DD', validTo: validTo(60) },
        }));

        problems.forEach((problem) => {
          expect(problem.getSolution()).to.not.match(/dashmate restart/);
        });
      });
    });

    it('should not advise a restart when the disk copy has itself expired', () => {
      const problems = analyse(served({
        certificate: { fingerprint256: 'AA:BB', validTo: validTo(-40) },
        matchesOnDisk: false,
        onDisk: { fingerprint256: 'CC:DD', validTo: validTo(-10) },
      }));

      problems.forEach((problem) => {
        expect(problem.getSolution()).to.not.match(/dashmate restart/);
      });
    });
  });

  // An external connect test measures whether something is listening, and
  // nothing listens on port 80 on a healthy node except for the seconds a
  // renewal takes. So it reports CLOSED on healthy nodes by construction, and
  // attaching it to every certificate problem sends operators to rewrite
  // firewall rules that are already correct.
  it('should not claim port 80 is unreachable from a listener probe', () => {
    samples.setServiceInfo('gateway', 'validationHttpPort', 'CLOSED');

    const problems = analyse(served({ certificate: { fingerprint256: 'AA:BB', validTo: validTo(-1) } }));

    expect(problems).to.have.length.greaterThan(0);
    problems.forEach((problem) => {
      expect(problem.getDescription()).to.not.match(/port 80 is not reachable/i);
    });
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
    // Doctor collects a wire sample too, and the two can legitimately disagree:
    // a gateway running with a healthy in-memory certificate and a stale bundle
    // on disk is exactly the case this analyser exists alongside. So the
    // on-disk problem states what the files show, not what a client would do.
    it('should not state a client outcome from an on-disk check', () => {
      const [problem] = analyseInstalled({
        status: 'INVALID',
        reasons: [{ code: 'SWITCH_INCOMPLETE', message: 'a switch was interrupted' }],
        warnings: [],
      });

      expect(problem.getSolution()).to.not.match(/clients? rejects?/i);
      expect(problem.getSolution()).to.not.match(/clients (are|were|could not|cannot|unable)/i);
      expect(problem.getSolution()).to.contain('not usable');
    });

    // `dashmate ssl obtain` signals the gateway itself once it has the files,
    // so telling the operator to restart afterwards buys an outage and nothing
    // else. Restart guidance belongs on the paths where nothing else reloads.
    it('should not ask for a restart after a command that reloads by itself', () => {
      const problems = analyseInstalled({
        status: 'INVALID',
        reasons: [{ code: 'EXPIRED', message: 'expired' }],
        warnings: [{ code: 'EXPIRING_SOON', message: 'expires tomorrow' }],
      });

      expect(problems).to.have.lengthOf(2);
      problems.forEach((problem) => {
        expect(problem.getSolution()).to.contain('dashmate ssl obtain');
        expect(problem.getSolution()).to.not.match(/dashmate restart/);
      });
    });

    it('should say that updates still deliver images', () => {
      const [problem] = analyseInstalled({
        status: 'INVALID',
        reasons: [{ code: 'EXPIRED', message: 'expired' }],
        warnings: [],
      });

      expect(problem.getSolution()).to.include('Updates still work');
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

  describe('when the installed certificate cannot be reinstated', () => {
    // One report must not prescribe two different commands for one certificate.
    // Each remedy below is reached by a different combination of what the wire
    // serves and what is on disk, and an operator reading a forced command
    // beside an unforced one has no way to tell which one their node needs.
    const SERVED_STATES = {
      'a sound certificate': served(),
      'an expired certificate matching the disk copy': served({
        certificate: { fingerprint256: 'AA:BB', validTo: validTo(-1) },
      }),
      'an expired certificate the disk copy differs from': served({
        certificate: { fingerprint256: 'CC:DD', validTo: validTo(-1) },
        matchesOnDisk: false,
        onDisk: { fingerprint256: 'AA:BB', validTo: validTo(-5) },
      }),
      'a live certificate the disk copy differs from': served({
        matchesOnDisk: false,
        onDisk: { fingerprint256: 'EE:FF', validTo: validTo(-5) },
      }),
    };

    Object.entries(SERVED_STATES).forEach(([state, servedCertificate]) => {
      it(`should force every repair it prescribes while serving ${state}`, () => {
        // Carries a reason that reinstalling cannot fix alongside a warning:
        // a stale-address certificate that is also close to expiry is the
        // ordinary shape of the problem, not a contrived one.
        samples.setServiceInfo('gateway', 'installedCertificate', {
          status: 'INVALID',
          reasons: [{
            code: 'IP_MISMATCH',
            message: 'The certificate is not valid for this address.',
          }],
          warnings: [{
            code: 'EXPIRING_SOON',
            message: 'The installed certificate expires in less than 7 days.',
          }],
          fingerprint256: 'AA:BB',
        });

        const prescribed = analyse(servedCertificate)
          .map((problem) => problem.getSolution())
          .filter((solution) => solution.includes('ssl obtain'));

        expect(prescribed).not.to.be.empty();
        prescribed.forEach((solution) => expect(solution).to.include('--force'));
      });
    });
  });
});
