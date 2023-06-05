const path = require('path');
const os = require('os');
const fs = require('fs');
const publicIp = require('public-ip');

process.env.DASHMATE_HOME_DIR = path.resolve(os.tmpdir(), '.dashmate');

const { asValue } = require('awilix');

const createDIContainer = require('../../src/createDIContainer');
const areServicesRunningFactory = require('../../src/test/areServicesRunningFactory');
const { SERVICES } = require('../../src/test/constants/services');
const { NODE_TYPE_NAMES, getNodeTypeByName } = require('../../src/listr/tasks/setup/nodeTypes');
const { SSL_PROVIDERS } = require('../../src/constants');
const generateTenderdashNodeKey = require("../../src/tenderdash/generateTenderdashNodeKey");
const getSelfSignedCertificate = require("../../src/test/getSelfSignedCertificate");
const isServiceRunningFactory = require("../../src/test/isServiceRunningFactory");
const wait = require("../../src/util/wait");
const createRpcClient = require('../../src/core/createRpcClient');

describe('Testnet Fullnode', function main() {
  this.timeout(60 * 60 * 1000); // 60 minutes

  let container;
  let setupRegularPresetTask;
  let resetNodeTask;
  let group;
  let configFile;
  let startGroupNodesTask;
  let dockerCompose;
  let areServicesRunning;
  let stopNodeTask;
  let restartNodeTask;
  let startNodeTask;
  let isServiceRunning;
  let coreRpcClient;
  let lastBlockHeight;

  const preset = 'testnet';

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

    setupRegularPresetTask = container.resolve('setupRegularPresetTask');
    resetNodeTask = container.resolve('resetNodeTask');
    startGroupNodesTask = container.resolve('startGroupNodesTask');
    startNodeTask = container.resolve('startNodeTask');
    restartNodeTask = container.resolve('restartNodeTask');
    stopNodeTask = container.resolve('stopNodeTask');
    const configFileRepository = container.resolve('configFileRepository');

    dockerCompose = container.resolve('dockerCompose');

    areServicesRunning = areServicesRunningFactory(group, dockerCompose, SERVICES);

    const setupTask = setupRegularPresetTask();

    const initialIp = await publicIp.v4();

    const { certificatePath, privKeyPath } = await getSelfSignedCertificate(initialIp);

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
    });

    configFile = container.resolve('configFile');

    const config = configFile.getConfig(preset);

    const serviceConfigFiles = renderServiceTemplates(config);
    writeServiceConfigs(config.getName(), serviceConfigFiles);

    await configFileRepository.write(configFile);

    isServiceRunning = isServiceRunningFactory(
      config,
      dockerCompose,
      SERVICES,
    );

    coreRpcClient = createRpcClient({
      port: config.get('core.rpc.port'),
      user: config.get('core.rpc.user'),
      pass: config.get('core.rpc.password'),
    });
  });

  after(async () => {
    if (fs.existsSync(process.env.DASHMATE_HOME_DIR)) {
      const config = configFile.getConfig(preset);

      const resetTask = resetNodeTask(config);

      await resetTask.run({
        isHardReset: false,
        isForce: false,
      });

      await configFile.removeConfig(config.getName());
    }
  });

  it('#setup', async () => {
    const configExists = configFile.getConfig(preset);

    expect(configExists).to.not.be.undefined();
  });

  it('#start', async () => {
    const startTask = startNodeTask(configFile.getConfig(preset));
    await startTask.run();

    const isRunning = await isServiceRunning('core');

    expect(isRunning).to.be.true();
  });

  it('#sync', async () => {
    await wait(360000);

    let blockchainInfo = await coreRpcClient.getBlockchainInfo();

    if (blockchainInfo.result.headers.length === 0) {
      expect.fail('Core is not syncing');
    }

    lastBlockHeight = blockchainInfo.result.headers;

    await wait(120000);

    blockchainInfo = await coreRpcClient.getBlockchainInfo();

    if (blockchainInfo.result.headers <= lastBlockHeight) {
      expect.fail('Core is not syncing');
    }

    lastBlockHeight = blockchainInfo.result.headers;
  });

  it('#restart', async () => {
    const task = restartNodeTask(configFile.getConfig(preset));
    await task.run();

    const isRunning = await isServiceRunning('core');

    expect(isRunning).to.be.true();

    await wait(120000);

    let blockchainInfo = await coreRpcClient.getBlockchainInfo();

    if (blockchainInfo.result.headers <= lastBlockHeight) {
      expect.fail('Core is not syncing after restart');
    }
  });

  it('#stop', async () => {
    const task = stopNodeTask(configFile.getConfig(preset));
    await task.run();

    const isRunning = await isServiceRunning('core');

    expect(isRunning).to.be.false();
  });
});
