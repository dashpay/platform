import probeActivationState, {
  ACTIVATION_REASON,
  ACTIVATION_STATE,
  probeConfigFormatMigrated,
  probeHelperRestarted,
} from '../../../src/update/probeActivationState.js';
import getConfigMock from '../../../src/test/mock/getConfigMock.js';

const CONFIG_FILE_PATH = '/home/dash/.dashmate/config.json';
const DASHMATE_VERSION = '4.1.0';
const HELPER_IMAGE = `dashpay/dashmate-helper:${DASHMATE_VERSION}`;
const OLD_HELPER_IMAGE = 'dashpay/dashmate-helper:4.0.0';

/**
 * @param {string} configFormatVersion
 * @return {string}
 */
function configFileJson(configFormatVersion) {
  return JSON.stringify({
    configFormatVersion,
    projectId: 'abcdef12',
    defaultConfigName: 'mainnet',
    configs: {},
  });
}

/**
 * @param {string} image
 * @param {string} [state]
 * @return {Object}
 */
function helperContainer(image, state = 'running') {
  return {
    Service: 'dashmate_helper',
    Name: 'dashmate_helper',
    Image: image,
    State: state,
  };
}

/**
 * @param {string} code
 * @return {Error}
 */
function fsError(code) {
  const error = new Error(`${code}: failed`);

  error.code = code;

  return error;
}

describe('probeActivationState', () => {
  let readFile;
  let getConfigFileMigrations;
  let dockerCompose;
  let config;

  /**
   * @param {Object} [overrides]
   * @return {Promise<Object>}
   */
  function probeMigration(overrides = {}) {
    return probeConfigFormatMigrated({
      configFilePath: CONFIG_FILE_PATH,
      getConfigFileMigrations,
      dashmateVersion: DASHMATE_VERSION,
      readFile,
      ...overrides,
    });
  }

  /**
   * @param {Object} [overrides]
   * @return {Promise<Object>}
   */
  function probeHelper(overrides = {}) {
    return probeHelperRestarted(config, {
      dockerCompose,
      expectedHelperImage: HELPER_IMAGE,
      ...overrides,
    });
  }

  beforeEach(function beforeEach() {
    readFile = this.sinon.stub().resolves(configFileJson(DASHMATE_VERSION));

    // Migrations are keyed at the release they ship in, so the newest key decides the format
    // this build produces
    getConfigFileMigrations = this.sinon.stub().returns({
      '0.24.0': () => {},
      '4.1.0': () => {},
    });

    dockerCompose = {
      getContainersList: this.sinon.stub().resolves([helperContainer(HELPER_IMAGE)]),
    };

    config = getConfigMock(this.sinon);
    config.getName.returns('mainnet');
  });

  describe('migration probe', () => {
    it('should report activated when the config file on disk carries the current format', async () => {
      const probe = await probeMigration();

      expect(probe.state).to.equal(ACTIVATION_STATE.ACTIVATED);
      expect(probe.reason).to.equal(ACTIVATION_REASON.CONFIG_FORMAT_CURRENT);
      expect(probe.onDiskVersion).to.equal(DASHMATE_VERSION);
      expect(probe.expectedVersion).to.equal(DASHMATE_VERSION);
      expect(readFile).to.have.been.calledOnceWith(CONFIG_FILE_PATH);
    });

    it('should report pending when the config file on disk was written by an older dashmate', async () => {
      readFile.resolves(configFileJson('4.0.0'));

      const probe = await probeMigration();

      expect(probe.state).to.equal(ACTIVATION_STATE.PENDING);
      expect(probe.reason).to.equal(ACTIVATION_REASON.CONFIG_FORMAT_BEHIND);
      expect(probe.onDiskVersion).to.equal('4.0.0');
    });

    it('should report pending when a migration is keyed ahead of the package version', async () => {
      // A development build carries migrations above its own version, and the config format it
      // produces is that newest migration rather than package.json
      getConfigFileMigrations.returns({ '4.2.0': () => {} });

      const probe = await probeMigration();

      expect(probe.state).to.equal(ACTIVATION_STATE.PENDING);
      expect(probe.expectedVersion).to.equal('4.2.0');
    });

    it('should report unknown when the config file is in a format newer than this build produces', async () => {
      readFile.resolves(configFileJson('4.2.0'));

      const probe = await probeMigration();

      // A downgrade: migration returns early on such a file, so this is not work that
      // re-running this binary can complete
      expect(probe.state).to.equal(ACTIVATION_STATE.UNKNOWN);
      expect(probe.reason).to.equal(ACTIVATION_REASON.CONFIG_FORMAT_AHEAD);
      expect(probe.onDiskVersion).to.equal('4.2.0');
    });

    it('should report unknown when the config file does not exist', async () => {
      readFile.rejects(fsError('ENOENT'));

      const probe = await probeMigration();

      expect(probe.state).to.equal(ACTIVATION_STATE.UNKNOWN);
      expect(probe.reason).to.equal(ACTIVATION_REASON.CONFIG_FILE_MISSING);
    });

    it('should report unknown when the config file cannot be read', async () => {
      readFile.rejects(fsError('EACCES'));

      const probe = await probeMigration();

      expect(probe.state).to.equal(ACTIVATION_STATE.UNKNOWN);
      expect(probe.reason).to.equal(ACTIVATION_REASON.CONFIG_FILE_UNREADABLE);
    });

    it('should report unknown when the config file is not valid JSON', async () => {
      readFile.resolves('{ "configFormatVersion": ');

      const probe = await probeMigration();

      expect(probe.state).to.equal(ACTIVATION_STATE.UNKNOWN);
      expect(probe.reason).to.equal(ACTIVATION_REASON.CONFIG_FILE_MALFORMED);
    });

    it('should report unknown when the config file carries no configFormatVersion', async () => {
      readFile.resolves(JSON.stringify({ configs: {} }));

      const probe = await probeMigration();

      expect(probe.state).to.equal(ACTIVATION_STATE.UNKNOWN);
      expect(probe.reason).to.equal(ACTIVATION_REASON.CONFIG_FORMAT_VERSION_INVALID);
    });

    it('should report unknown when configFormatVersion is not a version', async () => {
      readFile.resolves(configFileJson('latest'));

      const probe = await probeMigration();

      expect(probe.state).to.equal(ACTIVATION_STATE.UNKNOWN);
      expect(probe.reason).to.equal(ACTIVATION_REASON.CONFIG_FORMAT_VERSION_INVALID);
    });

    it('should report unknown when the migrations cannot be listed', async () => {
      getConfigFileMigrations.throws(new Error('no default configs'));

      const probe = await probeMigration();

      expect(probe.state).to.equal(ACTIVATION_STATE.UNKNOWN);
      expect(probe.reason).to.equal(ACTIVATION_REASON.PROBE_FAILED);
    });

    it('should decide without docker so it can run on cheap paths', async () => {
      const probe = await probeMigration();

      expect(probe.state).to.equal(ACTIVATION_STATE.ACTIVATED);
      expect(dockerCompose.getContainersList).to.have.not.been.called();
    });
  });

  describe('helper probe', () => {
    it('should report activated when the helper runs the image this dashmate pins', async () => {
      const probe = await probeHelper();

      expect(probe.state).to.equal(ACTIVATION_STATE.ACTIVATED);
      expect(probe.reason).to.equal(ACTIVATION_REASON.HELPER_IMAGE_MATCHES);
      expect(probe.runningImage).to.equal(HELPER_IMAGE);
      expect(dockerCompose.getContainersList).to.have.been.calledOnceWith(
        config,
        { filterServiceNames: 'dashmate_helper' },
      );
    });

    it('should report pending when the helper still runs the previous version image', async () => {
      dockerCompose.getContainersList.resolves([helperContainer(OLD_HELPER_IMAGE)]);

      const probe = await probeHelper();

      expect(probe.state).to.equal(ACTIVATION_STATE.PENDING);
      expect(probe.reason).to.equal(ACTIVATION_REASON.HELPER_IMAGE_MISMATCH);
      expect(probe.runningImage).to.equal(OLD_HELPER_IMAGE);
      expect(probe.expectedImage).to.equal(HELPER_IMAGE);
    });

    it('should report pending when no helper container is present', async () => {
      dockerCompose.getContainersList.resolves([]);

      const probe = await probeHelper();

      expect(probe.state).to.equal(ACTIVATION_STATE.PENDING);
      expect(probe.reason).to.equal(ACTIVATION_REASON.HELPER_NOT_RUNNING);
    });

    it('should report pending when the helper container is present but stopped', async () => {
      dockerCompose.getContainersList.resolves([helperContainer(HELPER_IMAGE, 'exited')]);

      const probe = await probeHelper();

      expect(probe.state).to.equal(ACTIVATION_STATE.PENDING);
      expect(probe.reason).to.equal(ACTIVATION_REASON.HELPER_NOT_RUNNING);
    });

    it('should report pending when any running helper is left on an old image', async () => {
      dockerCompose.getContainersList.resolves([
        helperContainer(HELPER_IMAGE),
        helperContainer(OLD_HELPER_IMAGE),
      ]);

      const probe = await probeHelper();

      expect(probe.state).to.equal(ACTIVATION_STATE.PENDING);
      expect(probe.reason).to.equal(ACTIVATION_REASON.HELPER_IMAGE_MISMATCH);
      expect(probe.runningImage).to.equal(OLD_HELPER_IMAGE);
    });

    it('should report unknown without asking docker when the helper is built from sources', async () => {
      config.get.withArgs('dashmate.helper.docker.build.enabled').returns(true);

      const probe = await probeHelper();

      // The build override runs dashmate-helper:local, a name that never changes, so it can
      // never witness an upgrade. That is not the operator's pending work
      expect(probe.state).to.equal(ACTIVATION_STATE.UNKNOWN);
      expect(probe.reason).to.equal(ACTIVATION_REASON.HELPER_BUILT_LOCALLY);
      expect(dockerCompose.getContainersList).to.have.not.been.called();
    });

    it('should report unknown when docker cannot be reached', async () => {
      dockerCompose.getContainersList.rejects(new Error('Cannot connect to the Docker daemon'));

      const probe = await probeHelper();

      expect(probe.state).to.equal(ACTIVATION_STATE.UNKNOWN);
      expect(probe.reason).to.equal(ACTIVATION_REASON.DOCKER_UNAVAILABLE);
      expect(probe.message).to.contain('Cannot connect to the Docker daemon');
    });

    it('should report unknown when docker returns no usable container list', async () => {
      dockerCompose.getContainersList.resolves(undefined);

      const probe = await probeHelper();

      expect(probe.state).to.equal(ACTIVATION_STATE.UNKNOWN);
      expect(probe.reason).to.equal(ACTIVATION_REASON.DOCKER_UNAVAILABLE);
    });

    it('should report unknown when a running helper reports no image', async () => {
      dockerCompose.getContainersList.resolves([helperContainer(undefined)]);

      const probe = await probeHelper();

      expect(probe.state).to.equal(ACTIVATION_STATE.UNKNOWN);
      expect(probe.reason).to.equal(ACTIVATION_REASON.HELPER_IMAGE_UNREADABLE);
    });

    it('should report unknown when reading the build option throws', async () => {
      config.get.withArgs('dashmate.helper.docker.build.enabled')
        .throws(new Error('option path not found'));

      const probe = await probeHelper();

      expect(probe.state).to.equal(ACTIVATION_STATE.UNKNOWN);
      expect(probe.reason).to.equal(ACTIVATION_REASON.PROBE_FAILED);
    });

    it('should report unknown when it is given no docker compose', async () => {
      const probe = await probeHelper({ dockerCompose: undefined });

      expect(probe.state).to.equal(ACTIVATION_STATE.UNKNOWN);
      expect(probe.reason).to.equal(ACTIVATION_REASON.PROBE_FAILED);
    });
  });

  describe('combined', () => {
    /**
     * @param {Object} [overrides]
     * @return {Promise<Object>}
     */
    function probe(overrides = {}) {
      return probeActivationState(config, {
        configFilePath: CONFIG_FILE_PATH,
        getConfigFileMigrations,
        dashmateVersion: DASHMATE_VERSION,
        readFile,
        dockerCompose,
        expectedHelperImage: HELPER_IMAGE,
        ...overrides,
      });
    }

    it('should report activated only when both probes agree', async () => {
      const activation = await probe();

      expect(activation.state).to.equal(ACTIVATION_STATE.ACTIVATED);
      expect(activation.migration.state).to.equal(ACTIVATION_STATE.ACTIVATED);
      expect(activation.helper.state).to.equal(ACTIVATION_STATE.ACTIVATED);
    });

    it('should report pending when the helper is still on the old image', async () => {
      dockerCompose.getContainersList.resolves([helperContainer(OLD_HELPER_IMAGE)]);

      const activation = await probe();

      expect(activation.state).to.equal(ACTIVATION_STATE.PENDING);
      expect(activation.reason).to.equal(ACTIVATION_REASON.HELPER_IMAGE_MISMATCH);
    });

    it('should prefer definite pending evidence over an unknown from the other probe', async () => {
      readFile.resolves(configFileJson('4.0.0'));
      dockerCompose.getContainersList.rejects(new Error('Cannot connect to the Docker daemon'));

      const activation = await probe();

      // The config file is provably behind, so the node is not activated whatever docker was
      // unable to say
      expect(activation.state).to.equal(ACTIVATION_STATE.PENDING);
      expect(activation.reason).to.equal(ACTIVATION_REASON.CONFIG_FORMAT_BEHIND);
      expect(activation.helper.state).to.equal(ACTIVATION_STATE.UNKNOWN);
    });

    it('should report unknown when no probe found anything definite', async () => {
      config.get.withArgs('dashmate.helper.docker.build.enabled').returns(true);
      readFile.rejects(fsError('ENOENT'));

      const activation = await probe();

      expect(activation.state).to.equal(ACTIVATION_STATE.UNKNOWN);
      expect(activation.reason).to.equal(ACTIVATION_REASON.CONFIG_FILE_MISSING);
    });

    it('should not read the config file again when a migration probe is passed in', async () => {
      const migration = await probeMigration();

      readFile.resetHistory();

      const activation = await probe({ migration });

      expect(activation.state).to.equal(ACTIVATION_STATE.ACTIVATED);
      expect(readFile).to.have.not.been.called();
    });

    it('should never throw when it is given nothing to probe with', async () => {
      const activation = await probeActivationState(undefined, {});

      expect(activation.state).to.equal(ACTIVATION_STATE.UNKNOWN);
      expect(activation.migration.reason).to.equal(ACTIVATION_REASON.PROBE_FAILED);
      expect(activation.helper.reason).to.equal(ACTIVATION_REASON.PROBE_FAILED);
    });
  });
});
