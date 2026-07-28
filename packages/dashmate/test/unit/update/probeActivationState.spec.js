import { readFile as readFileFromDisk } from 'fs/promises';

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
const LOCAL_HELPER_IMAGE = 'dashmate-helper:local';

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

    it('should order 4.9.0 before 4.10.0 rather than comparing versions as text', async () => {
      getConfigFileMigrations.returns({ '4.10.0': () => {} });
      readFile.resolves(configFileJson('4.9.0'));

      const probe = await probeMigration({ dashmateVersion: '4.10.0' });

      // Read as text, "4.9.0" sorts after "4.10.0" and this host would be told it is on a
      // format newer than the binary produces
      expect(probe.state).to.equal(ACTIVATION_STATE.PENDING);
      expect(probe.reason).to.equal(ACTIVATION_REASON.CONFIG_FORMAT_BEHIND);
    });

    it('should order 4.10.0 after 4.9.0 rather than comparing versions as text', async () => {
      getConfigFileMigrations.returns({ '4.9.0': () => {} });
      readFile.resolves(configFileJson('4.10.0'));

      const probe = await probeMigration({ dashmateVersion: '4.9.0' });

      expect(probe.state).to.equal(ACTIVATION_STATE.UNKNOWN);
      expect(probe.reason).to.equal(ACTIVATION_REASON.CONFIG_FORMAT_AHEAD);
    });

    it('should report pending when the config file is at a prerelease of the format this build produces', async () => {
      readFile.resolves(configFileJson('4.1.0-rc.1'));

      const probe = await probeMigration();

      // A prerelease precedes its release, though as text it sorts after it
      expect(probe.state).to.equal(ACTIVATION_STATE.PENDING);
      expect(probe.reason).to.equal(ACTIVATION_REASON.CONFIG_FORMAT_BEHIND);
    });

    it('should report activated when the config format differs only by build metadata', async () => {
      readFile.resolves(configFileJson('4.1.0+build.9'));

      const probe = await probeMigration();

      // Build metadata is ignored by version precedence, though as text it sorts after
      expect(probe.state).to.equal(ACTIVATION_STATE.ACTIVATED);
      expect(probe.reason).to.equal(ACTIVATION_REASON.CONFIG_FORMAT_CURRENT);
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

    it('should not throw when it is given no options at all', async () => {
      const probe = await probeConfigFormatMigrated(null);

      expect(probe.state).to.equal(ACTIVATION_STATE.UNKNOWN);
      expect(probe.reason).to.equal(ACTIVATION_REASON.PROBE_FAILED);
    });

    it('should not throw when reading the config file rejects with something that is not an error', async () => {
      readFile.rejects(null);

      const probe = await probeMigration();

      expect(probe.state).to.equal(ACTIVATION_STATE.UNKNOWN);
      expect(probe.reason).to.equal(ACTIVATION_REASON.CONFIG_FILE_UNREADABLE);
    });

    it('should not depend on the docker layer, so the cheap probe cannot start spawning processes', async () => {
      // The module's own source is the only place this boundary is visible: an implementation
      // that reached for a docker singleton directly would still satisfy every behavioural
      // test above, because none of them can inject what they are never handed
      const source = await readFileFromDisk(
        new URL('../../../src/update/probeActivationState.js', import.meta.url),
        'utf8',
      );

      const dependencies = [...source.matchAll(/(?:from|import)\s*\(?\s*'([^']+)'/g)]
        .map(([, specifier]) => specifier);

      expect(dependencies).to.have.length.greaterThan(0);
      expect(dependencies.filter((specifier) => /docker/i.test(specifier))).to.be.empty();
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

    it('should report pending when the running tag only starts with the expected one', async () => {
      dockerCompose.getContainersList.resolves([helperContainer(`${HELPER_IMAGE}-rc.1`)]);

      const probe = await probeHelper();

      // A prerelease of the expected tag is a different image, and a comparison that accepted
      // it would accept every tag the expected one is a prefix of
      expect(probe.state).to.equal(ACTIVATION_STATE.PENDING);
      expect(probe.reason).to.equal(ACTIVATION_REASON.HELPER_IMAGE_MISMATCH);
    });

    it('should report pending when the running tag only contains the expected one', async () => {
      dockerCompose.getContainersList.resolves([helperContainer(`docker.io/${HELPER_IMAGE}`)]);

      // Compose reports the image as the compose file declares it, so a reference that merely
      // contains the expected tag is not the image compose would create now
      const probe = await probeHelper();

      expect(probe.state).to.equal(ACTIVATION_STATE.PENDING);
      expect(probe.reason).to.equal(ACTIVATION_REASON.HELPER_IMAGE_MISMATCH);
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

    it('should report pending when a helper on an old image runs beside one with no image', async () => {
      dockerCompose.getContainersList.resolves([
        helperContainer(undefined),
        helperContainer(OLD_HELPER_IMAGE),
      ]);

      // A known old image settles the question; letting the unreadable one decide would
      // downgrade conclusive evidence to "could not determine"
      const probe = await probeHelper();

      expect(probe.state).to.equal(ACTIVATION_STATE.PENDING);
      expect(probe.reason).to.equal(ACTIVATION_REASON.HELPER_IMAGE_MISMATCH);
      expect(probe.runningImage).to.equal(OLD_HELPER_IMAGE);
    });

    it('should report unknown when the running helper was built from local sources', async () => {
      dockerCompose.getContainersList.resolves([helperContainer(LOCAL_HELPER_IMAGE)]);

      const probe = await probeHelper();

      // The build override runs dashmate-helper:local, a name that never changes, so it can
      // never witness an upgrade. That is not the operator's pending work
      expect(probe.state).to.equal(ACTIVATION_STATE.UNKNOWN);
      expect(probe.reason).to.equal(ACTIVATION_REASON.HELPER_BUILT_LOCALLY);
      expect(probe.runningImage).to.equal(LOCAL_HELPER_IMAGE);
    });

    it('should report pending when an old released helper runs beside a locally built one', async () => {
      dockerCompose.getContainersList.resolves([
        helperContainer(LOCAL_HELPER_IMAGE),
        helperContainer(OLD_HELPER_IMAGE),
      ]);

      const probe = await probeHelper();

      expect(probe.state).to.equal(ACTIVATION_STATE.PENDING);
      expect(probe.reason).to.equal(ACTIVATION_REASON.HELPER_IMAGE_MISMATCH);
    });

    it('should decide from the running container rather than from what the config asks for', async () => {
      config.get.withArgs('dashmate.helper.docker.build.enabled').returns(true);
      dockerCompose.getContainersList.resolves([helperContainer(OLD_HELPER_IMAGE)]);

      const probe = await probeHelper();

      // Turning the build option on does not retire the released helper that is still up, and
      // the container is what the node is actually running
      expect(probe.state).to.equal(ACTIVATION_STATE.PENDING);
      expect(probe.reason).to.equal(ACTIVATION_REASON.HELPER_IMAGE_MISMATCH);
      expect(config.get).to.have.not.been.called();
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

    it('should report unknown when the only running helper reports no image', async () => {
      dockerCompose.getContainersList.resolves([helperContainer(undefined)]);

      const probe = await probeHelper();

      expect(probe.state).to.equal(ACTIVATION_STATE.UNKNOWN);
      expect(probe.reason).to.equal(ACTIVATION_REASON.HELPER_IMAGE_UNREADABLE);
    });

    it('should report unknown when a helper with no image runs beside one on the expected tag', async () => {
      dockerCompose.getContainersList.resolves([
        helperContainer(HELPER_IMAGE),
        helperContainer(undefined),
      ]);

      // Nothing contradicts the expected image, but the container that cannot be read could be
      // the stale one, so this is not proof of activation
      const probe = await probeHelper();

      expect(probe.state).to.equal(ACTIVATION_STATE.UNKNOWN);
      expect(probe.reason).to.equal(ACTIVATION_REASON.HELPER_IMAGE_UNREADABLE);
    });

    it('should report unknown when it is given no docker compose', async () => {
      const probe = await probeHelper({ dockerCompose: undefined });

      expect(probe.state).to.equal(ACTIVATION_STATE.UNKNOWN);
      expect(probe.reason).to.equal(ACTIVATION_REASON.PROBE_FAILED);
    });

    it('should not throw when it is given no config and no options at all', async () => {
      const probe = await probeHelperRestarted(null, null);

      expect(probe.state).to.equal(ACTIVATION_STATE.UNKNOWN);
      expect(probe.reason).to.equal(ACTIVATION_REASON.PROBE_FAILED);
    });

    it('should not throw when listing containers rejects with something that is not an error', async () => {
      dockerCompose.getContainersList.rejects(null);

      const probe = await probeHelper();

      expect(probe.state).to.equal(ACTIVATION_STATE.UNKNOWN);
      expect(probe.reason).to.equal(ACTIVATION_REASON.DOCKER_UNAVAILABLE);
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
      dockerCompose.getContainersList.resolves([helperContainer(LOCAL_HELPER_IMAGE)]);
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

    it('should report unknown when a passed in migration result carries a state it does not recognise', async () => {
      const activation = await probe({
        migration: { state: 'bogus', reason: 'whatever', message: 'bad' },
      });

      // A cache that has been truncated, hand-edited or written by another version must never
      // be able to certify activation
      expect(activation.state).to.equal(ACTIVATION_STATE.UNKNOWN);
      expect(activation.reason).to.equal(ACTIVATION_REASON.PROBE_FAILED);
      expect(activation.migration.state).to.equal(ACTIVATION_STATE.UNKNOWN);
    });

    it('should report unknown when a passed in migration result is missing its reason', async () => {
      const activation = await probe({ migration: { state: ACTIVATION_STATE.ACTIVATED } });

      expect(activation.state).to.equal(ACTIVATION_STATE.UNKNOWN);
      expect(activation.reason).to.equal(ACTIVATION_REASON.PROBE_FAILED);
    });

    it('should report unknown when a passed in migration result is not an object', async () => {
      const activation = await probe({ migration: ACTIVATION_STATE.ACTIVATED });

      expect(activation.state).to.equal(ACTIVATION_STATE.UNKNOWN);
      expect(activation.reason).to.equal(ACTIVATION_REASON.PROBE_FAILED);
    });

    it('should never throw when it is given nothing to probe with', async () => {
      const activation = await probeActivationState(undefined, {});

      expect(activation.state).to.equal(ACTIVATION_STATE.UNKNOWN);
      expect(activation.migration.reason).to.equal(ACTIVATION_REASON.PROBE_FAILED);
      expect(activation.helper.reason).to.equal(ACTIVATION_REASON.PROBE_FAILED);
    });

    it('should never throw when it is given no options at all', async () => {
      const activation = await probeActivationState(null, null);

      expect(activation.state).to.equal(ACTIVATION_STATE.UNKNOWN);
      expect(activation.reason).to.equal(ACTIVATION_REASON.PROBE_FAILED);
    });
  });
});
