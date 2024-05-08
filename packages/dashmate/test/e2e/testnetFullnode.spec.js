import { asValue } from 'awilix';

import { NODE_TYPE_NAMES, getNodeTypeByName } from '../../src/listr/tasks/setup/nodeTypes.js';
import { SSL_PROVIDERS } from '../../src/constants.js';
import HomeDir from '../../src/config/HomeDir.js';
import createDIContainer from '../../src/createDIContainer.js';
import createSelfSignedCertificate from '../../src/test/createSelfSignedCertificate.js';
import generateTenderdashNodeKey from '../../src/tenderdash/generateTenderdashNodeKey.js';
import createRpcClient from '../../src/core/createRpcClient.js';
import waitForCoreDataFactory from '../../src/test/waitForCoreDataFactory.js';

describe('Testnet Fullnode', function main() {
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

      if (process.env.DASHMATE_E2E_TESTS_SKIP_IMAGE_BUILD !== 'true') {
        config.set('dashmate.helper.docker.build.enabled', true);
        config.set('platform.drive.abci.docker.build.enabled', true);
        config.set('platform.dapi.api.docker.build.enabled', true);
      }

      config.set('docker.network.subnet', '172.27.24.0/24');
      config.set('dashmate.helper.api.port', 40000);
      config.set('core.p2p.port', 40001);
      config.set('core.rpc.port', 40002);
      config.set('platform.gateway.listeners.dapiAndDrive.port', 40003);
      config.set('platform.drive.tenderdash.p2p.port', 40004);
      config.set('platform.drive.tenderdash.rpc.port', 40005);
      config.set('platform.drive.tenderdash.pprof.port', 40006);

      // Write configs
      await configFileRepository.write(configFile);

      const writeConfigTemplates = container.resolve('writeConfigTemplates');
      writeConfigTemplates(config);
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
        host: config.get('core.rpc.host'),
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

  describe('reset', () => {
    it('should reset fullnode', async () => {
      const resetNodeTask = container.resolve('resetNodeTask');
      const resetTask = resetNodeTask(config);

      await resetTask.run({
        isHardReset: false,
        isForce: true,
        isVerbose: true,
      });

      homeDir.remove();
    });
  });
});
