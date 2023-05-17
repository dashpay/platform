const path = require('path');
const os = require('os');
const fs = require('fs');

process.env.DASHMATE_HOME_DIR = path.resolve(os.tmpdir(), '.dashmate');

const { asValue } = require('awilix');

const createDIContainer = require('../../src/createDIContainer');

describe('Local HP Masternode', function main() {
  this.timeout(100000);

  let container;
  let setupLocalPresetTask;

  before(async () => {
    container = await createDIContainer();

    const createSystemConfigs = container.resolve('createSystemConfigs');

    const configFile = createSystemConfigs();

    container.register({
      configFile: asValue(configFile),
    });

    const defaultGroupName = configFile.getDefaultGroupName();

    const group = configFile.getGroupConfigs(defaultGroupName);

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
  });

  after(async () => {
    await fs.removeSync(process.env.DASHMATE_HOME_DIR, { recursive: true, force: true });
  });

  it('#setup', async () => {
    const task = setupLocalPresetTask();

    await task.run({
      nodeCount: 3,
      debugLogs: true,
      minerInterval: '2.5m',
    });
  });
});
