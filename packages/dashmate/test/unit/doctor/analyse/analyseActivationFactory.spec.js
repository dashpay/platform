import analyseActivationFactory from '../../../../src/doctor/analyse/analyseActivationFactory.js';
import { SEVERITY } from '../../../../src/doctor/Prescription.js';
import Samples from '../../../../src/doctor/Samples.js';
import { ACTIVATION_REASON, ACTIVATION_STATE } from '../../../../src/update/probeActivationState.js';

const CURRENT_HELPER = 'dashpay/dashmate-helper:5.2.0';
const OLD_HELPER = 'dashpay/dashmate-helper:5.1.0';

const MIGRATED = {
  state: ACTIVATION_STATE.ACTIVATED,
  reason: ACTIVATION_REASON.CONFIG_FORMAT_CURRENT,
  message: 'Config file is migrated to 5.2.0',
};

/**
 * @param {string} name
 * @param {Object} [overrides]
 * @return {Object} a sampled config, as the collector records it
 */
function activatedConfig(name, overrides = {}) {
  return {
    name,
    masternode: false,
    state: ACTIVATION_STATE.ACTIVATED,
    reason: ACTIVATION_REASON.HELPER_IMAGE_MATCHES,
    message: `Config file is migrated to 5.2.0, Helper container is running ${CURRENT_HELPER}`,
    helper: {
      state: ACTIVATION_STATE.ACTIVATED,
      reason: ACTIVATION_REASON.HELPER_IMAGE_MATCHES,
      message: `Helper container is running ${CURRENT_HELPER}`,
      expectedImage: CURRENT_HELPER,
      runningImage: CURRENT_HELPER,
    },
    ...overrides,
  };
}

/**
 * The state the check exists for: the package was upgraded and the node was never restarted,
 * so its helper still runs the image the previous dashmate pinned.
 *
 * @param {string} name
 * @param {Object} [overrides]
 * @return {Object}
 */
function pendingConfig(name, overrides = {}) {
  const message = `Helper container is running ${OLD_HELPER} instead of ${CURRENT_HELPER}`;

  return activatedConfig(name, {
    state: ACTIVATION_STATE.PENDING,
    reason: ACTIVATION_REASON.HELPER_IMAGE_MISMATCH,
    message,
    helper: {
      state: ACTIVATION_STATE.PENDING,
      reason: ACTIVATION_REASON.HELPER_IMAGE_MISMATCH,
      message,
      expectedImage: CURRENT_HELPER,
      runningImage: OLD_HELPER,
    },
    ...overrides,
  });
}

/**
 * @param {string} name
 * @param {boolean|null} nodeRunning
 * @return {Object}
 */
function helperDownConfig(name, nodeRunning) {
  const message = 'No running helper container was found for this config';

  return activatedConfig(name, {
    state: ACTIVATION_STATE.PENDING,
    reason: ACTIVATION_REASON.HELPER_NOT_RUNNING,
    message,
    helper: {
      state: ACTIVATION_STATE.PENDING,
      reason: ACTIVATION_REASON.HELPER_NOT_RUNNING,
      message,
      expectedImage: CURRENT_HELPER,
      runningImage: null,
    },
    nodeRunning,
  });
}

describe('analyseActivationFactory', () => {
  let analyseActivation;
  let samples;

  /**
   * @param {Object[]} configs
   * @param {Object} [migration]
   * @return {Problem[]}
   */
  function analyse(configs, migration = MIGRATED) {
    samples.setActivation({ dashmateVersion: '5.2.0', migration, configs });

    return analyseActivation(samples);
  }

  beforeEach(() => {
    samples = new Samples();

    analyseActivation = analyseActivationFactory();
  });

  it('should report a config whose helper still runs the image of the previous dashmate', () => {
    const problems = analyse([pendingConfig('mainnet')]);

    expect(problems).to.have.lengthOf(1);
    expect(problems[0].getSeverity()).to.equal(SEVERITY.MEDIUM);
    expect(problems[0].getDescription()).to.include('mainnet');
    expect(problems[0].getDescription()).to.include('5.2.0');
    expect(problems[0].getDescription()).to.include(OLD_HELPER);
    expect(problems[0].getSolution()).to.include('dashmate restart --config mainnet');
  });

  // Only a restart brings the node up on the installed dashmate; `dashmate update` pulls images
  // and leaves the old services running.
  it('should advise a restart rather than an update', () => {
    const [problem] = analyse([pendingConfig('mainnet')]);

    expect(problem.getSolution()).to.not.include('dashmate update');
  });

  // Without --safe the stop refuses inside a DKG window, so the copied command would fail on
  // exactly the nodes that must not miss a DKG.
  it('should restart a masternode in a DKG-safe window', () => {
    const [problem] = analyse([pendingConfig('mainnet', { masternode: true })]);

    expect(problem.getSolution()).to.include('dashmate restart --config mainnet --safe');
  });

  // A prerelease build reads a released helper as a different image, and telling the operator to
  // restart on it would take the node back to older services.
  it('should not advise a restart onto an older dashmate than the node runs', () => {
    const newer = 'dashpay/dashmate-helper:5.2.1';

    const [problem] = analyse([pendingConfig('mainnet', {
      helper: {
        state: ACTIVATION_STATE.PENDING,
        reason: ACTIVATION_REASON.HELPER_IMAGE_MISMATCH,
        message: `Helper container is running ${newer} instead of ${CURRENT_HELPER}`,
        expectedImage: CURRENT_HELPER,
        runningImage: newer,
      },
    })]);

    expect(problem.getSeverity()).to.equal(SEVERITY.MEDIUM);
    expect(problem.getSolution()).to.not.include('dashmate restart');
  });

  // The restart loads the certificate files on disk, which may be worse than what is served.
  it('should point at the certificate check before restarting a node that runs Platform', () => {
    const [problem] = analyse([pendingConfig('mainnet', { platform: true })]);

    expect(problem.getSolution()).to.include('dashmate doctor --config mainnet');
    expect(problem.getSolution()).to.include('dashmate restart --config mainnet');
  });

  // The config file is definite evidence; a missing helper must not demote it to unknown.
  it('should report a node with a config file behind as pending even with no helper running', () => {
    const [problem] = analyse([helperDownConfig('mainnet', true)].map((sampled) => ({
      ...sampled,
      reason: ACTIVATION_REASON.CONFIG_FORMAT_BEHIND,
      message: 'Config file is in format 5.1.0 and has not been migrated to 5.2.0',
    })));

    expect(problem.getSeverity()).to.equal(SEVERITY.MEDIUM);
    expect(problem.getSolution()).to.include('dashmate restart --config mainnet');
  });

  // Activated is the one verdict that silences the check, so it has to be confirmed by the
  // helper probe itself.
  it('should report unknown for an activated verdict the helper probe does not confirm', () => {
    const problems = analyse([activatedConfig('mainnet', { helper: undefined })]);

    expect(problems).to.have.lengthOf(1);
    expect(problems[0].getSeverity()).to.equal(SEVERITY.LOW);
  });

  it('should report nothing for a config that runs the installed dashmate', () => {
    expect(analyse([activatedConfig('mainnet')])).to.be.empty();
  });

  it('should report every pending config on the host, not only the first', () => {
    const problems = analyse([
      pendingConfig('mainnet'),
      activatedConfig('testnet'),
      pendingConfig('mainnet_2'),
    ]);

    expect(problems).to.have.lengthOf(2);
    expect(problems[0].getSolution()).to.include('dashmate restart --config mainnet');
    expect(problems[0].getSolution()).to.not.include('mainnet_2');
    expect(problems[1].getSolution()).to.include('dashmate restart --config mainnet_2');
  });

  // Doctor prints the command for an operator to paste, so a name a shell would split must
  // arrive quoted.
  it('should quote a config name a shell would split', () => {
    const [problem] = analyse([pendingConfig('my node')]);

    expect(problem.getSolution()).to.include('dashmate restart --config \'my node\'');
  });

  // A stopped node runs nothing old and starts on the installed version.
  it('should report nothing for a stopped config', () => {
    expect(analyse([helperDownConfig('testnet', false)])).to.be.empty();
  });

  // The rest of the node may still run what the previous dashmate started.
  it('should report unknown for a config whose helper is down while the node runs', () => {
    const problems = analyse([helperDownConfig('testnet', true)]);

    expect(problems).to.have.lengthOf(1);
    expect(problems[0].getSeverity()).to.equal(SEVERITY.LOW);
    expect(problems[0].getDescription()).to.include('testnet');
  });

  it('should report unknown when it could not tell whether a node without a helper runs', () => {
    expect(analyse([helperDownConfig('testnet', null)])).to.have.lengthOf(1);
  });

  it('should report unknown rather than nothing when the helper image cannot witness an upgrade', () => {
    const message = 'Helper container is running dashmate-helper:local, a tag that cannot witness an upgrade';

    const problems = analyse([activatedConfig('mainnet', {
      state: ACTIVATION_STATE.UNKNOWN,
      reason: ACTIVATION_REASON.HELPER_BUILT_LOCALLY,
      message,
      helper: {
        state: ACTIVATION_STATE.UNKNOWN,
        reason: ACTIVATION_REASON.HELPER_BUILT_LOCALLY,
        message,
      },
    })]);

    expect(problems).to.have.lengthOf(1);
    expect(problems[0].getSeverity()).to.equal(SEVERITY.LOW);
    expect(problems[0].getDescription()).to.include(message);
  });

  // A newer dashmate migrated the config file, so every node may run services newer than this
  // dashmate pins: telling the operator to restart them on it would take them backwards.
  it('should not advise restarting any node when the config file is ahead of this dashmate', () => {
    const message = 'Config file is in format 6.0.0, which is newer than the 5.2.0 this dashmate produces';
    const migration = {
      state: ACTIVATION_STATE.UNKNOWN,
      reason: ACTIVATION_REASON.CONFIG_FORMAT_AHEAD,
      message,
    };

    const problems = analyse([
      pendingConfig('mainnet'),
      pendingConfig('testnet'),
    ], migration);

    expect(problems).to.have.lengthOf(1);
    expect(problems[0].getSeverity()).to.equal(SEVERITY.MEDIUM);
    expect(problems[0].getDescription()).to.include(message);
    expect(problems[0].getSolution()).to.include('newer dashmate');
    expect(problems[0].getSolution()).to.not.include('dashmate restart');
  });

  // Every build from source shares one tag, so restarting can never settle the question.
  it('should not advise a restart for a helper built from source', () => {
    const message = 'Helper container is running dashmate-helper:local, a tag that cannot witness an upgrade';

    const [problem] = analyse([activatedConfig('mainnet', {
      state: ACTIVATION_STATE.UNKNOWN,
      reason: ACTIVATION_REASON.HELPER_BUILT_LOCALLY,
      message,
    })]);

    expect(problem.getSolution()).to.not.include('dashmate restart');
  });

  // A report collected by a future dashmate may carry a state this one has never seen; it must
  // not read as fine.
  it('should report a state it does not recognise as unknown', () => {
    const problems = analyse([activatedConfig('mainnet', { state: 'half-activated' })]);

    expect(problems).to.have.lengthOf(1);
    expect(problems[0].getSeverity()).to.equal(SEVERITY.LOW);
  });

  it('should report nothing more when docker is unavailable, which is reported on its own', () => {
    samples.setStringifiedDockerError('Docker is not running');
    samples.setActivation({ dashmateVersion: '5.2.0', error: 'Docker is not available' });

    expect(analyseActivation(samples)).to.be.empty();
  });

  it('should report a failed collection once', () => {
    samples.setActivation({ dashmateVersion: '5.2.0', error: 'Docker is not running' });

    const problems = analyseActivation(samples);

    expect(problems).to.have.lengthOf(1);
    expect(problems[0].getSeverity()).to.equal(SEVERITY.LOW);
    expect(problems[0].getDescription()).to.include('Docker is not running');
  });

  it('should report a malformed sample once instead of failing the analysis', () => {
    for (const activation of [
      'not an object',
      { dashmateVersion: '5.2.0', configs: 'not a list' },
      { dashmateVersion: '5.2.0', configs: [null] },
      { dashmateVersion: '5.2.0', configs: [{ state: ACTIVATION_STATE.PENDING }] },
    ]) {
      samples.setActivation(activation);

      const problems = analyseActivation(samples);

      expect(problems, JSON.stringify(activation)).to.have.lengthOf(1);
      expect(problems[0].getSeverity()).to.equal(SEVERITY.LOW);
    }
  });

  // A report collected by a dashmate that predates this check carries no verdict either way.
  it('should report nothing when no activation was sampled', () => {
    expect(analyseActivation(samples)).to.be.empty();
  });

  // A report is analysed wherever it is sent, so the version it names is the one that collected
  // it, not the one reading it.
  it('should name the dashmate version that collected the sample', () => {
    samples.setActivation({
      dashmateVersion: '9.9.9',
      migration: MIGRATED,
      configs: [pendingConfig('mainnet')],
    });

    const [problem] = analyseActivation(samples);

    expect(problem.getDescription()).to.include('9.9.9');
  });
});
