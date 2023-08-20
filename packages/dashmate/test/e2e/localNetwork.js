const path = require('path');
const os = require('os');
const fs = require('fs');

process.env.DASHMATE_HOME_DIR = path.resolve(os.tmpdir(), '.dashmate');

const { asValue } = require('awilix');

const createDIContainer = require('../../src/createDIContainer');
const areServicesRunningFactory = require('../../src/test/areServicesRunningFactory');
const { SERVICES } = require('../../src/test/constants/services');

describe.skip('Local Network', function main() {
  this.timeout(60 * 60 * 1000); // 60 minutes
  this.bail(true); // bail on first failure

  let container;
  let setupLocalPresetTask;
  let resetNodeTask;
  let group;
  let configFile;
  let startGroupNodesTask;
  let dockerCompose;
  let areServicesRunning;
  let stopNodeTask;
  let restartNodeTask;

  const groupName = 'local';

  before(async () => {
    container = await createDIContainer();

    const createSystemConfigs = container.resolve('createSystemConfigs');

    configFile = createSystemConfigs();

    container.register({
      configFile: asValue(configFile),
    });

    const defaultGroupName = configFile.getDefaultGroupName();

    group = configFile.getGroupConfigs(defaultGroupName);

    container.register({
      configGroup: asValue(group),
    });

    const renderServiceTemplates = container.resolve('renderServiceTemplates');
    const writeServiceConfigs = container.resolve('writeServiceConfigs');

    for (const config of group) {
      const serviceConfigFiles = renderServiceTemplates(config);
      writeServiceConfigs(config.getName(), serviceConfigFiles);
    }

    setupLocalPresetTask = await container.resolve('setupLocalPresetTask');
    resetNodeTask = await container.resolve('resetNodeTask');
    startGroupNodesTask = await container.resolve('startGroupNodesTask');
    restartNodeTask = await container.resolve('restartNodeTask');
    stopNodeTask = await container.resolve('stopNodeTask');

    dockerCompose = await container.resolve('dockerCompose');
  });

  after(async () => {
    if (fs.existsSync(process.env.DASHMATE_HOME_DIR)) {
      for (const config of group) {
        const resetTask = resetNodeTask(config);

        await resetTask.run({
          isHardReset: false,
          isForce: false,
        });

        await configFile.removeConfig(config.getName());
      }
    }
  });

  describe('setup', () => {
    it('should setup local network', async () => {
      const setupTask = setupLocalPresetTask();

      await setupTask.run({
        nodeCount: 3,
        debugLogs: true,
        minerInterval: '2.5m',
        isVerbose: true,
      });

      configFile = container.resolve('configFile');

      const configExists = configFile.isGroupExists(groupName);

      group = configFile.getGroupConfigs(groupName);

      areServicesRunning = areServicesRunningFactory(configFile, group, dockerCompose, SERVICES);

      expect(configExists).to.be.true();
    });
  });

  describe('start', () => {
    it('should start local network', async () => {
      const task = startGroupNodesTask(group);

      await task.run();

      const result = await areServicesRunning();

      expect(result).to.be.true();
    });
  });

  describe('restart', () => {
    it('should restart local network', async () => {
      for (const config of group) {
        const task = restartNodeTask(config);
        await task.run();
      }

      const result = await areServicesRunning();

      expect(result).to.be.true();
    });
  });

  describe('stop', () => {
    it('should stop local network', async () => {
      for (const config of group.reverse()) {
        const task = stopNodeTask(config);
        await task.run();
      }

      const result = await areServicesRunning();

      expect(result).to.be.false();
    });
  });
});
