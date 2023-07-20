const { asValue } = require('awilix');

const createDIContainer = require('../../src/createDIContainer');
const HomeDir = require('../../src/config/HomeDir');

describe('Local Network', function main() {
  this.timeout(60 * 60 * 1000); // 60 minutes
  this.bail(true); // bail on first failure

  let homeDir;
  let container;
  let configGroup;
  let configFile;
  let assertLocalServicesRunning;

  const groupName = 'local';

  before(async () => {
    container = await createDIContainer();

    homeDir = container.resolve('homeDir');
    homeDir.change(HomeDir.createTemp());

    // Create config file
    const createSystemConfigs = container.resolve('createSystemConfigs');

    configFile = createSystemConfigs();

    // Enable dashmate helper docker build for local group
    const localConfig = configFile.getConfig(groupName);
    localConfig.set('dashmate.helper.docker.build.enabled', true);

    container.register({
      configFile: asValue(configFile),
    });

    assertLocalServicesRunning = container.resolve('assertLocalServicesRunning');
  });

  after(async () => {
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

  describe('setup', () => {
    it('should setup local network', async () => {
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

      // Store config group for further usage
      configGroup = configFile.getGroupConfigs(groupName);

      container.register({
        configGroup: asValue(configGroup),
      });

      // Write service configs
      const renderServiceTemplates = container.resolve('renderServiceTemplates');
      const writeServiceConfigs = container.resolve('writeServiceConfigs');

      for (const config of configGroup) {
        config.set('dashmate.helper.docker.build.enabled', true);

        const serviceConfigFiles = renderServiceTemplates(config);
        writeServiceConfigs(config.getName(), serviceConfigFiles);
      }
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
});
