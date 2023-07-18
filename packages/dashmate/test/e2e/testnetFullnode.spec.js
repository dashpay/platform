const path = require('path');
const os = require('os');
const fs = require('fs');
const publicIp = require('public-ip');

const { asValue } = require('awilix');

const constants = require('../../src/constants');

const createDIContainer = require('../../src/createDIContainer');
const { NODE_TYPE_NAMES, getNodeTypeByName } = require('../../src/listr/tasks/setup/nodeTypes');
const { SSL_PROVIDERS } = require('../../src/constants');
const generateTenderdashNodeKey = require('../../src/tenderdash/generateTenderdashNodeKey');
const createSelfSignedCertificate = require('../../src/test/createSelfSignedCertificate');
const assertServiceRunningFactory = require('../../src/test/asserts/assertServiceRunningFactory');
const createRpcClient = require('../../src/core/createRpcClient');
const waitForCoreDataFactory = require('../../src/test/waitForCoreDataFactory');

describe.skip('Testnet Fullnode', function main() {
  this.timeout(60 * 60 * 1000); // 60 minutes
  this.bail(true); // bail on first failure

  let container;
  let setupRegularPresetTask;
  let resetNodeTask;
  let config;
  let configFile;
  let dockerCompose;
  let stopNodeTask;
  let restartNodeTask;
  let startNodeTask;
  let assertServiceRunning;
  let coreRpcClient;
  let lastBlockHeight;
  let renderServiceTemplates;
  let writeServiceConfigs;
  let configFileRepository;
  let waitForCoreData;

  const preset = 'testnet';

  before(async () => {
    constants.HOME_DIR_PATH = path.resolve(os.tmpdir(), '.dashmate');
    constants.CONFIG_FILE_PATH = path.join(constants.HOME_DIR_PATH, 'config.json');

    container = await createDIContainer();

    const createSystemConfigs = container.resolve('createSystemConfigs');

    configFile = createSystemConfigs();

    container.register({
      configFile: asValue(configFile),
    });

    renderServiceTemplates = container.resolve('renderServiceTemplates');
    writeServiceConfigs = container.resolve('writeServiceConfigs');

    setupRegularPresetTask = container.resolve('setupRegularPresetTask');
    resetNodeTask = container.resolve('resetNodeTask');
    startNodeTask = container.resolve('startNodeTask');
    restartNodeTask = container.resolve('restartNodeTask');
    stopNodeTask = container.resolve('stopNodeTask');
    configFileRepository = container.resolve('configFileRepository');

    dockerCompose = container.resolve('dockerCompose');

    configFile = container.resolve('configFile');

    assertServiceRunning = assertServiceRunningFactory(
      configFile,
      dockerCompose,
    );
  });

  after(async () => {
    if (!fs.existsSync(constants.HOME_DIR_PATH)) {
      return;
    }

    if (config) {
      const resetTask = resetNodeTask(config);

      await resetTask.run({
        isHardReset: false,
        isForce: true,
        isVerbose: true,
      });
    }

    fs.rmSync(constants.HOME_DIR_PATH, { recursive: true, force: true });
  });

  describe('setup', () => {
    it('should setup fullnode', async () => {
      // TODO: Refactor setup command to extract setup logic to
      //  setupTask function and use it here
      const setupTask = setupRegularPresetTask();

      const initialIp = await publicIp.v4();

      const { certificatePath, privKeyPath } = await createSelfSignedCertificate(initialIp);

      await setupTask.run({
        preset,
        nodeType: getNodeTypeByName(NODE_TYPE_NAMES.FULLNODE),
        isHP: false,
        certificateProvider: SSL_PROVIDERS.FILE,
        tenderdashNodeKey: generateTenderdashNodeKey(),
        initialIpForm: {
          ip: initialIp,
          coreP2PPort: 19999,
          platformHTTPPort: 36656,
          platformP2PPort: 1443,
        },
        fileCertificateProviderForm: {
          chainFilePath: certificatePath,
          privateFilePath: privKeyPath,
        },
        isVerbose: true,
      });

      config = configFile.getConfig(preset);

      const serviceConfigFiles = renderServiceTemplates(config);
      writeServiceConfigs(config.getName(), serviceConfigFiles);

      await configFileRepository.write(configFile);

      coreRpcClient = createRpcClient({
        port: config.get('core.rpc.port'),
        user: config.get('core.rpc.user'),
        pass: config.get('core.rpc.password'),
      });

      waitForCoreData = waitForCoreDataFactory(coreRpcClient);

      expect(config).to.not.be.undefined();
    });
  });

  describe('start', () => {
    it('should start fullnode', async () => {
      const startTask = startNodeTask(configFile.getConfig(preset));
      await startTask.run({
        isVerbose: true,
      });

      await assertServiceRunning(config, 'core');
    });
  });

  describe('sync', () => {
    it('should sync Dash Core', async () => {
      lastBlockHeight = await waitForCoreData(0, (currentValue) => currentValue > 0);
    });
  });

  describe('restart', () => {
    it('should restart fullnode and continue syncing Dash Core', async () => {
      const task = restartNodeTask(configFile.getConfig(preset));
      await task.run({
        isVerbose: true,
      });

      await assertServiceRunning(config, 'core');

      await waitForCoreData(
        lastBlockHeight,
        (currentValue, originalValue) => currentValue > originalValue,
      );
    });
  });

  describe('stop', () => {
    it('should stop fullnode', async () => {
      const task = stopNodeTask(configFile.getConfig(preset));
      await task.run({
        isVerbose: true,
      });

      await assertServiceRunning(config, 'core', false);
    });
  });
});
