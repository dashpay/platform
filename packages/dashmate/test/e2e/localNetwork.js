const path = require('path');
const os = require('os');
const fs = require('fs');

process.env.DASHMATE_HOME_DIR = path.resolve(os.tmpdir(), '.dashmate');

const { asValue } = require('awilix');

const createDIContainer = require('../../src/createDIContainer');
const areServicesRunningFactory = require('../../src/test/areServicesRunningFactory');
const { SERVICES } = require('../../src/test/constants/services');

describe('Local Network', function main() {
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

  const groupName = 'testnet';

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

    areServicesRunning = areServicesRunningFactory(group, dockerCompose, SERVICES);

    const setupTask = setupLocalPresetTask();

    await setupTask.run({
      nodeCount: 3,
      debugLogs: true,
      minerInterval: '2.5m',
      isVerbose: true,
    });
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

  it('setup', async () => {
    const configExists = configFile.isGroupExists(groupName);

    expect(configExists).to.be.true();
  });

  it('start', async () => {
    group = configFile.getGroupConfigs(groupName);

    areServicesRunning = areServicesRunningFactory(group, dockerCompose, SERVICES);

    for (const config of group) {
      config.set('platform.sourcePath', path.resolve(__dirname, '../../../../'));
    }

    const task = startGroupNodesTask(group);

    await task.run();

    const result = await areServicesRunning();

    expect(result).to.be.true();
  });

  it('restart', async () => {
    group = configFile.getGroupConfigs(groupName);

    areServicesRunning = areServicesRunningFactory(group, dockerCompose, SERVICES);

    for (const config of group) {
      const task = restartNodeTask(config);
      await task.run();
    }

    const result = await areServicesRunning();

    expect(result).to.be.true();
  });

  it('stop', async () => {
    group = configFile.getGroupConfigs(groupName);

    areServicesRunning = areServicesRunningFactory(group, dockerCompose, SERVICES);

    for (const config of group.reverse()) {
      const task = stopNodeTask(config);
      await task.run();
    }

    const result = await areServicesRunning();

    expect(result).to.be.false();
  });
});
