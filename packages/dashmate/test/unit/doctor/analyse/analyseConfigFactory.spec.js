import fs from 'fs';
import path from 'path';
import getBaseConfigFactory from '../../../../configs/defaults/getBaseConfigFactory.js';
import { PACKAGE_ROOT_DIR } from '../../../../src/constants.js';
import analyseConfigFactory from '../../../../src/doctor/analyse/analyseConfigFactory.js';
import { SEVERITY } from '../../../../src/doctor/Prescription.js';
import Samples from '../../../../src/doctor/Samples.js';
import { ERRORS as LETSENCRYPT_ERRORS } from '../../../../src/ssl/letsencrypt/validateLetsEncryptCertificateFactory.js';
import { ERRORS as ZEROSSL_ERRORS } from '../../../../src/ssl/zerossl/validateZeroSslCertificateFactory.js';
import deriveTenderdashNodeId from '../../../../src/tenderdash/deriveTenderdashNodeId.js';
import generateTenderdashNodeKey from '../../../../src/tenderdash/generateTenderdashNodeKey.js';

describe('analyseConfigFactory', () => {
  let analyseConfig;
  let config;
  let samples;

  /**
   * @param {Object} ssl
   * @param {string} [provider=zerossl]
   * @return {Problem[]}
   */
  function analyseSslSample(ssl, provider = 'zerossl') {
    config.set('platform.gateway.ssl.provider', provider);

    samples.setServiceInfo('gateway', 'ssl', ssl);

    return analyseConfig(samples);
  }

  beforeEach(() => {
    config = getBaseConfigFactory()();

    config.set('platform.enable', true);

    samples = new Samples();
    samples.setDashmateConfig(config);

    // Ports are reported healthy so that only certificate problems are analysed
    samples.setServiceInfo('core', 'p2pPort', 'OPEN');
    samples.setServiceInfo('gateway', 'httpPort', 'OPEN');
    samples.setServiceInfo('drive_tenderdash', 'p2pPort', 'OPEN');

    analyseConfig = analyseConfigFactory();
  });

  it('should report a problem for a Let\'s Encrypt certificate that expires soon', () => {
    const problems = analyseSslSample({
      error: LETSENCRYPT_ERRORS.CERTIFICATE_EXPIRES_SOON,
      data: { certificate: { expires: '2026-01-01' } },
    }, 'letsencrypt');

    expect(problems).to.have.lengthOf(1);
    expect(problems[0].getDescription()).to.include('Let\'s Encrypt certificate expires at');
    expect(problems[0].getSeverity()).to.equal(SEVERITY.HIGH);
  });

  // A contact address is optional and nothing prompts for one, so a node
  // without one has no problem to report. A doctor report is read to find
  // problems, and an entry saying "this is not a problem" is noise in it.
  //
  // Driven with the code an older dashmate could have recorded in an archive,
  // because doctor analyses those too: it must report nothing rather than
  // fail on a code it no longer knows.
  it('should report nothing for a node with no contact address', () => {
    const problems = analyseSslSample({ error: 'EMAIL_IS_NOT_SET', data: {} }, 'letsencrypt');

    expect(problems).to.be.empty();
  });

  // This fires when the issued certificate was never copied to where the gateway
  // loads from. A restart only makes the gateway re-read the copy it already
  // has, which is the out-of-date one - so on a node still serving a valid
  // certificate, following that advice is what takes it off the network.
  describe('a renewed certificate that never reached the gateway', () => {
    const notInstalled = () => analyseSslSample({
      error: LETSENCRYPT_ERRORS.CERTIFICATE_NOT_INSTALLED,
      data: {},
    }, 'letsencrypt');

    it('should not tell the operator to restart Platform', () => {
      const [problem] = notInstalled();

      expect(problem.getSolution()).to.not.match(/dashmate\s+restart/);
    });

    it('should tell the operator to install the issued certificate', () => {
      const [problem] = notInstalled();

      expect(problem.getSolution()).to.contain('dashmate ssl obtain');
    });

    // A host commonly runs several configs. A command pasted without one acts
    // on whichever happens to be the default, so it would obtain and reload a
    // certificate for a node nobody was diagnosing and leave this one as it is.
    it('should name the config being diagnosed', () => {
      const [problem] = notInstalled();

      expect(problem.getSolution()).to.contain(`--config ${config.getName()}`);
    });

    // The report can carry the gateway analyser's finding for the same node,
    // which says in as many words not to restart. Two opposite instructions in
    // one report leave the operator to guess, and one guess breaks the node.
    it('should not contradict the advice not to restart', () => {
      const [problem] = notInstalled();

      expect(problem.getSolution()).to.match(/not restart|Do not restart/);
    });
  });

  it('should report a problem for a ZeroSSL certificate that expires soon', () => {
    const problems = analyseSslSample({
      error: ZEROSSL_ERRORS.CERTIFICATE_EXPIRES_SOON,
      data: { certificate: { expires: '2026-01-01' } },
    });

    expect(problems).to.have.lengthOf(1);
    expect(problems[0].getDescription()).to.include('ZeroSSL certificate expires at');
    expect(problems[0].getSeverity()).to.equal(SEVERITY.HIGH);
  });

  it('should report a problem when the ZeroSSL API key is not set', () => {
    const problems = analyseSslSample({
      error: ZEROSSL_ERRORS.API_KEY_IS_NOT_SET,
      data: {},
    });

    expect(problems).to.have.lengthOf(1);
    expect(problems[0].getDescription()).to.include('ZeroSSL API key is not set');
    expect(problems[0].getSolution()).to.include('dashmate config set platform.gateway.ssl.providerConfigs.zerossl.apiKey');
  });

  it('should report a problem when the external IP is not set', () => {
    const problems = analyseSslSample({
      error: ZEROSSL_ERRORS.EXTERNAL_IP_IS_NOT_SET,
      data: {},
    });

    expect(problems).to.have.lengthOf(1);
    expect(problems[0].getDescription()).to.include('External IP is not set');
    expect(problems[0].getSolution()).to.include('dashmate config set externalIp');
  });

  it('should report a problem when certificate files are not found', () => {
    const problems = analyseSslSample({
      error: 'not-exist',
      data: {
        chainFilePath: '/home/dashmate/ssl/bundle.crt',
        privateFilePath: '/home/dashmate/ssl/private.key',
      },
    }, 'file');

    expect(problems).to.have.lengthOf(1);
    expect(problems[0].getDescription()).to.include('SSL certificate files are not found');
  });

  describe('ZeroSSL remediation', () => {
    it('should offer both renewing with ZeroSSL and switching to Let\'s Encrypt', () => {
      // Whether renewing works depends on the operator's ZeroSSL plan, which dashmate cannot
      // see, so both routes are offered rather than one being asserted to be the answer.
      const [problem] = analyseSslSample({
        error: ZEROSSL_ERRORS.CERTIFICATE_EXPIRES_SOON,
        data: { certificate: { expires: '2026-01-01' } },
      });

      expect(problem.getSolution()).to.include('dashmate ssl obtain');
      expect(problem.getSolution()).to.include('platform.gateway.ssl.provider letsencrypt');
    });

    it('should name what makes the alternative worth taking', () => {
      // Both providers renew on their own, so that is not what separates them
      const [problem] = analyseSslSample({
        error: ZEROSSL_ERRORS.CERTIFICATE_EXPIRES_SOON,
        data: { certificate: { expires: '2026-01-01' } },
      });

      expect(problem.getSolution()).to.include('free');
    });

    it('should surface the reason ZeroSSL itself gave', () => {
      const [problem] = analyseSslSample({
        error: ZEROSSL_ERRORS.ZERO_SSL_API_ERROR,
        data: { error: { message: 'Limit of certificates on your ZeroSSL account was reached' } },
      });

      expect(problem.getDescription()).to.include('Limit of certificates');
      expect(problem.getSolution()).to.include('platform.gateway.ssl.provider letsencrypt');
    });

    it('should still report an API failure that carried no message', () => {
      // The description doubles as the presence check, so an empty one dropped the problem
      const problems = analyseSslSample({
        error: ZEROSSL_ERRORS.ZERO_SSL_API_ERROR,
        data: {},
      });

      expect(problems).to.have.lengthOf(1);
    });

    it('should not suggest a command that does not exist', () => {
      const [problem] = analyseSslSample({
        error: ZEROSSL_ERRORS.CERTIFICATE_IS_NOT_VALID,
        data: {},
      });

      expect(problem.getSolution()).to.not.include('ssl zerossl obtain');
    });
  });

  it('should not report a problem for a valid certificate', () => {
    const problems = analyseSslSample({ data: {} });

    expect(problems).to.be.empty();
  });

  // These checks predate the renewal record and each ends in its own request.
  // They run before the renewal-aware analyser in the same report, so a node
  // whose recorded cause forbids asking again would read "do not obtain" from
  // one and a runnable command from the other - and follow the command.
  describe('when the renewal record forbids another request', () => {
    /**
     * @param {Object} renewal
     * @return {Problem[]}
     */
    function analyseWithRecord(renewal) {
      samples.setServiceInfo('gateway', 'certificateRenewal', renewal);

      return analyseSslSample({
        error: 'CERTIFICATE_EXPIRES_SOON',
        data: { certificate: { expires: '2026-01-01' } },
      }, 'letsencrypt');
    }

    it('should withhold its own request when an issuance is outstanding', () => {
      const [problem] = analyseWithRecord({
        state: 'PRESENT',
        provider: 'letsencrypt',
        outcome: 'failed',
        code: 'CERTIFICATE_ISSUED_NOT_SAVED',
        attemptedAt: new Date().toISOString(),
        consecutiveFailures: 1,
        issuanceSpentAt: new Date().toISOString(),
      });

      expect(problem.getSolution()).to.not.contain('ssl obtain');
      expect(problem.getSolution()).to.contain('could not be saved');
    });

    it('should withhold it when the record cannot be read', () => {
      const [problem] = analyseWithRecord({ state: 'UNREADABLE', error: 'not json' });

      expect(problem.getSolution()).to.not.contain('ssl obtain');
    });

    // Quota and plan failures produce a provider switch, not an outright
    // refusal. These remedies ask the same provider for another certificate,
    // while the renewal-aware analyser in the same report says that provider
    // will never issue one again.
    it('should withhold its own request when the provider must be switched', () => {
      const [problem] = analyseWithRecord({
        state: 'PRESENT',
        provider: 'letsencrypt',
        outcome: 'failed',
        code: 'QUOTA_EXHAUSTED',
        attemptedAt: new Date().toISOString(),
        consecutiveFailures: 1,
      });

      expect(problem.getSolution()).to.contain('--provider letsencrypt');
    });

    // The configuration watcher hands over without clearing the old provider's
    // record, so a stale one must not suppress a request that is now valid.
    it('should ignore a record left by a provider no longer in use', () => {
      const [problem] = analyseWithRecord({
        state: 'PRESENT',
        provider: 'zerossl',
        outcome: 'failed',
        code: 'CERTIFICATE_ISSUED_NOT_SAVED',
        attemptedAt: new Date().toISOString(),
        consecutiveFailures: 1,
        issuanceSpentAt: new Date().toISOString(),
      });

      expect(problem.getSolution()).to.contain('ssl obtain');
    });

    // A certificate installed after the recorded failure overtakes it. The
    // renewal-aware analyser already ignores such a record; without the same
    // input here, this one would replace a valid repair with stale guidance
    // and the two would contradict each other in the same report.
    it('should ignore a failure a newer certificate has overtaken', () => {
      samples.setServiceInfo('gateway', 'installedCertificate', {
        validFrom: new Date(Date.now() - 60 * 60 * 1000).toISOString(),
      });

      const [problem] = analyseWithRecord({
        state: 'PRESENT',
        provider: 'letsencrypt',
        outcome: 'failed',
        code: 'CERTIFICATE_ISSUED_NOT_SAVED',
        // Two hours ago, so the installed certificate came after it.
        attemptedAt: new Date(Date.now() - 2 * 60 * 60 * 1000).toISOString(),
        consecutiveFailures: 1,
        issuanceSpentAt: new Date(Date.now() - 2 * 60 * 60 * 1000).toISOString(),
      });

      expect(problem.getSolution()).to.contain('ssl obtain');
    });

    it('should still print it when nothing forbids one', () => {
      const [problem] = analyseWithRecord({ state: 'ABSENT' });

      expect(problem.getSolution()).to.contain('ssl obtain');
    });
  });

  describe('Platform node ID', () => {
    const NODE_ID_PROBLEM = 'Platform Node ID doesn\'t match the one found in the ProReg transaction';

    let masternodeStatus;

    beforeEach(() => {
      // Captured from a Dash Core v24.0.3 node running a registered evonode
      masternodeStatus = JSON.parse(fs.readFileSync(
        path.join(PACKAGE_ROOT_DIR, 'test/fixtures/coreRpc/24.0.3/masternode-status-evonode.json'),
        'utf8',
      ));

      samples.setServiceInfo('core', 'masternodeStatus', masternodeStatus);
    });

    // Tenderdash identifies itself to its peers with the configured key, and
    // they only accept it as this evonode when its ID is the one registered on
    // chain. A node whose key does not match runs but never takes part.
    it('should report a node key that does not match the registered node ID', () => {
      const nodeKey = generateTenderdashNodeKey();

      config.set('platform.drive.tenderdash.node.key', nodeKey);
      config.set('platform.drive.tenderdash.node.id', deriveTenderdashNodeId(nodeKey));

      const descriptions = analyseConfig(samples).map((problem) => problem.getDescription());

      expect(descriptions).to.include(NODE_ID_PROBLEM);
    });

    it('should report nothing when the node key gives the registered ID and it is saved', () => {
      samples.setServiceInfo('drive_tenderdash', 'nodeIdFromKey', masternodeStatus.dmnState.platformNodeID);
      config.set('platform.drive.tenderdash.node.id', masternodeStatus.dmnState.platformNodeID);

      expect(analyseConfig(samples)).to.be.empty();
    });

    // dashmate starts Platform of a masternode only while the saved node ID is
    // the one its key gives, and `config set` of the key alone keeps the old
    // ID. The running node is fine; its next start or restart is refused.
    it('should name the node ID to save when only the node key gives the registered one', () => {
      const registeredId = masternodeStatus.dmnState.platformNodeID;
      samples.setServiceInfo('drive_tenderdash', 'nodeIdFromKey', registeredId);
      config.set('platform.drive.tenderdash.node.id', 'f'.repeat(40));

      const problems = analyseConfig(samples);

      expect(problems).to.have.lengthOf(1);
      expect(problems[0].getDescription()).to.not.equal(NODE_ID_PROBLEM);
      expect(problems[0].getSolution())
        .to.include(`dashmate config set platform.drive.tenderdash.node.id ${registeredId}`)
        .and.not.match(/dashmate\s+restart/);
      expect(problems[0].getSeverity()).to.equal(SEVERITY.MEDIUM);
    });

    it('should analyse a config with neither a node key nor a saved node ID', () => {
      config.set('platform.drive.tenderdash.node', {});

      const descriptions = analyseConfig(samples).map((problem) => problem.getDescription());

      expect(descriptions).to.include(NODE_ID_PROBLEM);
    });

    describe('while Tenderdash runs with another node ID', () => {
      let tenderdashStatus;

      beforeEach(() => {
        // Captured from Tenderdash 2.0.0-dev.3, and stored the way the
        // collector stores it: as a settled promise
        tenderdashStatus = JSON.parse(fs.readFileSync(
          path.join(PACKAGE_ROOT_DIR, 'test/fixtures/tenderdashRpc/2.0.0-dev.3/status.json'),
          'utf8',
        ));

        samples.setServiceInfo('drive_tenderdash', 'status', { status: 'fulfilled', value: tenderdashStatus });
        samples.setServiceInfo('drive_tenderdash', 'nodeIdFromKey', masternodeStatus.dmnState.platformNodeID);
        config.set('platform.drive.tenderdash.node.id', masternodeStatus.dmnState.platformNodeID);
      });

      // Tenderdash loads its key at startup, so a key corrected in the config
      // only takes effect after a restart. Until then peers still see the old ID.
      it('should say a restart is needed when the configured key is right but not yet loaded', () => {
        const problems = analyseConfig(samples);

        expect(problems).to.have.lengthOf(1);
        expect(problems[0].getDescription()).to.not.equal(NODE_ID_PROBLEM);
        expect(problems[0].getDescription()).to.include('Tenderdash is running with a node ID');
        expect(problems[0].getSolution()).to.include('dashmate restart --platform');
        expect(problems[0].getSeverity()).to.equal(SEVERITY.HIGH);
      });

      // The restart is refused while the saved ID is not the one the key
      // gives, so the ID is saved first.
      it('should name the node ID to save before the restart when the saved one is stale', () => {
        const registeredId = masternodeStatus.dmnState.platformNodeID;
        config.set('platform.drive.tenderdash.node.id', 'f'.repeat(40));

        const problems = analyseConfig(samples);

        expect(problems).to.have.lengthOf(1);
        expect(problems[0].getDescription()).to.include('Tenderdash is running with a node ID');

        const solution = problems[0].getSolution();
        const saveAt = solution.indexOf(`dashmate config set platform.drive.tenderdash.node.id ${registeredId}`);

        expect(saveAt).to.be.at.least(0);
        expect(solution.indexOf('dashmate restart --platform')).to.be.above(saveAt);
      });

      // Without a key there is nothing for a restarted Tenderdash to load, so
      // a restart would stop a running node that then cannot start again
      it('should not advise a restart when only the saved node ID matches and there is no key', () => {
        samples.setServiceInfo('drive_tenderdash', 'nodeIdFromKey', undefined);
        config.set('platform.drive.tenderdash.node', { id: masternodeStatus.dmnState.platformNodeID });

        const problems = analyseConfig(samples);

        expect(problems).to.have.lengthOf(1);
        expect(problems[0].getDescription()).to.include('Tenderdash is running with a node ID');
        expect(problems[0].getSolution()).to.not.match(/dashmate\s+restart/);
        expect(problems[0].getSolution()).to.include('platform.drive.tenderdash.node.key');
        expect(problems[0].getSeverity()).to.equal(SEVERITY.HIGH);
      });

      it('should report nothing when Tenderdash runs with the registered ID', () => {
        tenderdashStatus.result.node_info.id = masternodeStatus.dmnState.platformNodeID;

        expect(analyseConfig(samples)).to.be.empty();
      });

      it('should report nothing extra when Tenderdash did not answer', () => {
        samples.setServiceInfo('drive_tenderdash', 'status', { status: 'rejected', reason: {} });

        expect(analyseConfig(samples)).to.be.empty();
      });
    });

    // A report collected by an older dashmate has no ID derived from the key
    it('should report nothing when the node ID is the registered one', () => {
      config.set('platform.drive.tenderdash.node.id', masternodeStatus.dmnState.platformNodeID);

      expect(analyseConfig(samples)).to.be.empty();
    });
  });

  describe('gateway TLS private key permissions', () => {
    it('should report a problem when the private key is readable by other users', () => {
      samples.setServiceInfo('gateway', 'sslPrivateKeyMode', 0o644);

      const problem = analyseConfig(samples)
        .find((item) => item.getDescription().includes('private key'));

      expect(problem).to.exist();
      expect(problem.getDescription()).to.include('600');
      expect(problem.getSolution()).to.include('chmod 600');
      expect(problem.getSeverity()).to.equal(SEVERITY.HIGH);
    });

    it('should report a problem when the private key is readable by the group', () => {
      samples.setServiceInfo('gateway', 'sslPrivateKeyMode', 0o640);

      expect(analyseConfig(samples)
        .find((item) => item.getDescription().includes('private key'))).to.exist();
    });

    it('should report nothing when the private key is accessible to its owner only', () => {
      samples.setServiceInfo('gateway', 'sslPrivateKeyMode', 0o600);

      expect(analyseConfig(samples)
        .find((item) => item.getDescription().includes('private key'))).to.be.undefined();
    });

    // Doctor analyses archives collected by an older dashmate, which recorded
    // no mode at all, and a node whose key has not been obtained yet has none
    it('should report nothing when no private key mode was collected', () => {
      expect(analyseConfig(samples)
        .find((item) => item.getDescription().includes('private key'))).to.be.undefined();
    });
  });
});
