import { asValue } from 'awilix';
import createDIContainer from '../../src/createDIContainer.js';
import HomeDir from '../../src/config/HomeDir.js';

describe('Local Network', function main() {
  this.timeout(60 * 60 * 1000); // 60 minutes
  this.bail(true); // bail on first failure

  let homeDir;
  let container;
  let configGroup;
  let configFile;
  let configFileRepository;
  let assertLocalServicesRunning;

  const groupName = 'local';

  before(async () => {
    container = await createDIContainer();

    homeDir = container.resolve('homeDir');
    if (process.env.DASHMATE_E2E_TESTS_LOCAL_HOMEDIR) {
      homeDir.change(new HomeDir(process.env.DASHMATE_E2E_TESTS_LOCAL_HOMEDIR));
    } else {
      homeDir.change(HomeDir.createTemp());
    }

    // Create config file
    /**
     * @type {ConfigFileJsonRepository}
     */
    configFileRepository = container.resolve('configFileRepository');

    const createConfigFile = container.resolve('createConfigFile');

    if (process.env.DASHMATE_E2E_TESTS_LOCAL_HOMEDIR) {
      configFile = configFileRepository.read();
    } else {
      configFile = createConfigFile();
    }

    // Update local config template that will be used to setup nodes
    const localConfig = configFile.getConfig(groupName);

    if (process.env.DASHMATE_E2E_TESTS_SKIP_IMAGE_BUILD !== 'true') {
      localConfig.set('dashmate.helper.docker.build.enabled', true);
      localConfig.set('platform.drive.abci.docker.build.enabled', true);
      localConfig.set('platform.dapi.api.docker.build.enabled', true);
    }

    localConfig.set('docker.network.subnet', '172.30.0.0/24');
    localConfig.set('dashmate.helper.api.port', 40000);
    localConfig.set('core.p2p.port', 40001);
    localConfig.set('core.rpc.port', 40002);
    localConfig.set('platform.gateway.listeners.dapiAndDrive.port', 40003);
    localConfig.set('platform.drive.tenderdash.p2p.port', 40004);
    localConfig.set('platform.drive.tenderdash.rpc.port', 40005);
    localConfig.set('platform.drive.tenderdash.pprof.port', 40006);

    container.register({
      configFile: asValue(configFile),
    });

    assertLocalServicesRunning = container.resolve('assertLocalServicesRunning');
  });

  describe('setup', () => {
    it('should setup local network', async function testSetup() {
      if (process.env.DASHMATE_E2E_TESTS_LOCAL_HOMEDIR) {
        this.skip('local network set up is provided');
      }

      // TODO: Refactor setup command to extract setup logic to
      //  setupTask function and use it here
      const setupLocalPresetTask = await container.resolve('setupLocalPresetTask');
      const setupTask = setupLocalPresetTask();

      await setupTask.run({
        nodeCount: 3,
        debugLogs: true,
        minerInterval: '2.5m',
        isVerbose: true,
      });

      const configExists = configFile.isGroupExists(groupName);

      expect(configExists).to.be.true();

      // Write configs
      await configFileRepository.write(configFile);

      const writeConfigTemplates = container.resolve('writeConfigTemplates');

      configGroup = configFile.getGroupConfigs(groupName);
      configGroup.forEach(writeConfigTemplates);
    });

    after(async () => {
      // Store config group for further usage
      configGroup = configFile.getGroupConfigs(groupName);

      container.register({
        configGroup: asValue(configGroup),
      });
    });
  });

  describe('start', () => {
    it('should start local network', async () => {
      const startGroupNodesTask = await container.resolve('startGroupNodesTask');
      const task = startGroupNodesTask(configGroup);

      await task.run({
        isVerbose: true,
        waitForReadiness: true,
      });

      await assertLocalServicesRunning(configGroup);
    });
  });

  describe('restart', () => {
    it('should restart local network', async () => {
      // TODO: Refactor group restart command to extract group restart logic
      //  to restartGroupNodesTask function and use it here
      const restartNodeTask = await container.resolve('restartNodeTask');

      for (const config of configGroup) {
        const task = restartNodeTask(config);
        await task.run({
          isVerbose: true,
        });
      }

      await assertLocalServicesRunning(configGroup);
    });
  });

  describe('stop', () => {
    it('should stop local network', async () => {
      // TODO: Refactor group stop command to extract group stop logic
      //  to restartGroupNodesTask function and use it here
      const stopNodeTask = await container.resolve('stopNodeTask');

      for (const config of configGroup.reverse()) {
        const task = stopNodeTask(config);
        await task.run({
          isVerbose: true,
        });
      }

      await assertLocalServicesRunning(configGroup, false);
    });
  });

  describe('reset', () => {
    it('should reset local network', async () => {
      const resetNodeTask = await container.resolve('resetNodeTask');

      for (const config of configFile.getGroupConfigs(groupName)) {
        const resetTask = resetNodeTask(config);

        await resetTask.run({
          isVerbose: true,
          isHardReset: false,
          isForce: true,
        });
      }

      homeDir.remove();
    });
  });
});
