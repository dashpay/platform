const path = require('path');
const os = require('os');
const fs = require('fs');

const { asValue } = require('awilix');

const constants = require('../../src/constants');

const createDIContainer = require('../../src/createDIContainer');
const { NODE_TYPE_NAMES, getNodeTypeByName } = require('../../src/listr/tasks/setup/nodeTypes');
const { SSL_PROVIDERS } = require('../../src/constants');
const generateTenderdashNodeKey = require('../../src/tenderdash/generateTenderdashNodeKey');
const createSelfSignedCertificate = require('../../src/test/createSelfSignedCertificate');
const createRpcClient = require('../../src/core/createRpcClient');
const waitForCoreDataFactory = require('../../src/test/waitForCoreDataFactory');

describe('Testnet Fullnode', function main() {
  this.timeout(60 * 60 * 1000); // 60 minutes
  this.bail(true); // bail on first failure

  let container;
  let config;
  let configFile;
  let assertServiceRunning;
  let lastBlockHeight;
  let waitForCoreData;

  const preset = 'testnet';

  before(async () => {
    constants.HOME_DIR_PATH = fs.mkdtempSync(path.join(os.tmpdir(), 'dashmate-'));
    constants.CONFIG_FILE_PATH = path.join(constants.HOME_DIR_PATH, 'config.json');

    container = await createDIContainer();

    const createSystemConfigs = container.resolve('createSystemConfigs');

    configFile = createSystemConfigs();

    container.register({
      configFile: asValue(configFile),
    });

    assertServiceRunning = container.resolve('assertServiceRunning');
  });

  after(async () => {
    if (!fs.existsSync(constants.HOME_DIR_PATH)) {
      return;
    }

    if (config) {
      const resetNodeTask = container.resolve('resetNodeTask');
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
      const setupRegularPresetTask = container.resolve('setupRegularPresetTask');
      const setupTask = setupRegularPresetTask();

      const ip = '127.0.0.1';

      const { certificatePath, privKeyPath } = await createSelfSignedCertificate(ip);

      await setupTask.run({
        preset,
        nodeType: getNodeTypeByName(NODE_TYPE_NAMES.FULLNODE),
        isHP: false,
        certificateProvider: SSL_PROVIDERS.FILE,
        tenderdashNodeKey: generateTenderdashNodeKey(),
        initialIpForm: {
          ip,
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

      const isConfigExists = configFile.isConfigExists(preset);

      expect(isConfigExists).to.be.true();

      config = configFile.getConfig(preset);

      config.set('dashmate.helper.docker.build.enabled', true);

      const renderServiceTemplates = container.resolve('renderServiceTemplates');
      const writeServiceConfigs = container.resolve('writeServiceConfigs');

      const serviceConfigFiles = renderServiceTemplates(config);
      writeServiceConfigs(config.getName(), serviceConfigFiles);
    });
  });

  describe('start', () => {
    it('should start fullnode', async () => {
      const startNodeTask = container.resolve('startNodeTask');

      const startTask = startNodeTask(configFile.getConfig(preset));

      await startTask.run({
        isVerbose: true,
      });

      await assertServiceRunning(config, 'core');
    });
  });

  describe('sync', () => {
    it('should sync Dash Core', async () => {
      const coreRpcClient = createRpcClient({
        port: config.get('core.rpc.port'),
        user: config.get('core.rpc.user'),
        pass: config.get('core.rpc.password'),
      });

      waitForCoreData = waitForCoreDataFactory(coreRpcClient);

      lastBlockHeight = await waitForCoreData(0, (currentValue) => currentValue > 0);
    });
  });

  describe('restart', () => {
    it('should restart fullnode and continue syncing Dash Core', async () => {
      const restartNodeTask = container.resolve('restartNodeTask');

      const task = restartNodeTask(config);

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
      const stopNodeTask = container.resolve('stopNodeTask');

      const task = stopNodeTask(config);

      await task.run({
        isVerbose: true,
      });

      await assertServiceRunning(config, 'core', false);
    });
  });
});
