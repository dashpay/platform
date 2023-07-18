const path = require('path');
const os = require('os');
const fs = require('fs');

const { asValue } = require('awilix');

const constants = require('../../src/constants');

const createDIContainer = require('../../src/createDIContainer');
const assertServiceRunningFactory = require('../../src/test/asserts/assertServiceRunningFactory');
const assertLocalServicesRunningFactory = require('../../src/test/asserts/assertLocalServicesRunningFactory');

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
  let assertLocalServicesRunning;
  let stopNodeTask;
  let restartNodeTask;

  const groupName = 'local';

  before(async () => {
    constants.HOME_DIR_PATH = path.resolve(os.tmpdir(), '.dashmate');
    constants.CONFIG_FILE_PATH = path.join(constants.HOME_DIR_PATH, 'config.json');

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

    const assertServiceRunning = assertServiceRunningFactory(
      configFile,
      dockerCompose,
    );

    assertLocalServicesRunning = assertLocalServicesRunningFactory(assertServiceRunning);
  });

  after(async () => {
    if (!fs.existsSync(constants.HOME_DIR_PATH)) {
      return;
    }

    for (const config of group) {
      const resetTask = resetNodeTask(config);

      await resetTask.run({
        isVerbose: true,
        isHardReset: false,
        isForce: true,
      });
    }

    fs.rmSync(constants.HOME_DIR_PATH, { recursive: true, force: true });
  });

  describe('setup', () => {
    it('should setup local network', async () => {
      // TODO: Refactor setup command to extract setup logic to
      //  setupTask function and use it here
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

      expect(configExists).to.be.true();
    });
  });

  describe('start', () => {
    it('should start local network', async () => {
      const task = startGroupNodesTask(group);

      await task.run({
        isVerbose: true,
        waitForReadiness: true,
      });

      await assertLocalServicesRunning(group);
    });
  });

  describe('restart', () => {
    it('should restart local network', async () => {
      // TODO: Refactor group restart command to extract group restart logic
      //  to restartGroupNodesTask function and use it here
      for (const config of group) {
        const task = restartNodeTask(config);
        await task.run({
          isVerbose: true,
        });
      }

      await assertLocalServicesRunning(group);
    });
  });

  describe('stop', () => {
    it('should stop local network', async () => {
      // TODO: Refactor group stop command to extract group stop logic
      //  to restartGroupNodesTask function and use it here
      for (const config of group.reverse()) {
        const task = stopNodeTask(config);
        await task.run({
          isVerbose: true,
        });
      }

      await assertLocalServicesRunning(group, false);
    });
  });
});
