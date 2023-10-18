const { asValue } = require('awilix');

const createDIContainer = require('../../src/createDIContainer');
const { NODE_TYPE_NAMES, getNodeTypeByName } = require('../../src/listr/tasks/setup/nodeTypes');
const { SSL_PROVIDERS } = require('../../src/constants');
const generateTenderdashNodeKey = require('../../src/tenderdash/generateTenderdashNodeKey');
const createSelfSignedCertificate = require('../../src/test/createSelfSignedCertificate');
const createRpcClient = require('../../src/core/createRpcClient');
const waitForCoreDataFactory = require('../../src/test/waitForCoreDataFactory');
const HomeDir = require('../../src/config/HomeDir');

describe('Testnet HP Fullnode', function main() {
  this.timeout(60 * 60 * 1000); // 60 minutes
  this.bail(true); // bail on first failure

  let homeDir;
  let container;
  let config;
  let configFile;
  let configFileRepository;
  let assertServiceRunning;
  let lastBlockHeight;
  let waitForCoreData;

  const preset = 'testnet';

  before(async () => {
    container = await createDIContainer();

    homeDir = container.resolve('homeDir');
    homeDir.change(HomeDir.createTemp());

    // Create config file
    configFileRepository = container.resolve('configFileRepository');

    const createConfigFile = container.resolve('createConfigFile');

    configFile = createConfigFile();

    container.register({
      configFile: asValue(configFile),
    });

    assertServiceRunning = container.resolve('assertServiceRunning');
  });

  after(async () => {
    if (config) {
      const resetNodeTask = container.resolve('resetNodeTask');
      const resetTask = resetNodeTask(config);

      await resetTask.run({
        isHardReset: false,
        isForce: true,
        isVerbose: true,
      });
    }

    homeDir.remove();
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
        isHP: true,
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
      config.set('platform.drive.abci.docker.build.enabled', true);
      config.set('docker.network.subnet', '172.27.24.0/24');
      config.set('dashmate.helper.api.port', 40000);
      config.set('core.p2p.port', 40001);
      config.set('core.rpc.port', 40002);
      config.set('platform.dapi.envoy.http.port', 40003);
      config.set('platform.drive.tenderdash.p2p.port', 40004);
      config.set('platform.drive.tenderdash.rpc.port', 40005);
      config.set('platform.drive.tenderdash.pprof.port', 40006);

      // Write configs
      await configFileRepository.write(configFile);

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

      // TODO: Assert all services are running
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

      // TODO: Assert all services are running
      await assertServiceRunning(config, 'core');

      await waitForCoreData(
        lastBlockHeight,
        (currentValue, originalValue) => currentValue > originalValue,
      );
    });
  });

  describe('stop', () => {
    it('should stop only platform', async () => {
      const stopNodeTask = container.resolve('stopNodeTask');
      const startNodeTask = container.resolve('startNodeTask');

      let task = stopNodeTask(config);

      await task.run({
        isVerbose: true,
        platformOnly: true
      });

      await assertServiceRunning(config, 'core', true);
      await assertServiceRunning(config, 'drive_abci', false);

      task = startNodeTask(config)

      await task.run({
        isVerbose: true,
        platformOnly: true
      });

      await assertServiceRunning(config, 'core', true);
      await assertServiceRunning(config, 'drive_abci', true);
    });

    it('should stop fullnode', async () => {
      const stopNodeTask = container.resolve('stopNodeTask');

      const task = stopNodeTask(config);

      await task.run({
        isVerbose: true,
      });

      // TODO: Assert all services are running
      await assertServiceRunning(config, 'core', false);
    });
  });
});
