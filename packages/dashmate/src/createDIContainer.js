const {
  createContainer: createAwilixContainer,
  InjectionMode,
  asFunction,
  asValue,
  asClass,
} = require('awilix');

const Docker = require('dockerode');

const DockerCompose = require('./docker/DockerCompose');
const StartedContainers = require('./docker/StartedContainers');
const stopAllContainersFactory = require('./docker/stopAllContainersFactory');

const startCoreFactory = require('./core/startCoreFactory');
const createRpcClient = require('./core/createRpcClient');
const waitForCoreStart = require('./core/waitForCoreStart');
const waitForCoreSync = require('./core/waitForCoreSync');
const waitForBlocks = require('./core/waitForBlocks');
const waitForConfirmations = require('./core/waitForConfirmations');
const generateBlsKeys = require('./core/generateBlsKeys');

const createNewAddress = require('./core/wallet/createNewAddress');
const generateBlocks = require('./core/wallet/generateBlocks');
const generateToAddress = require('./core/wallet/generateToAddress');
const importPrivateKey = require('./core/wallet/importPrivateKey');
const getAddressBalance = require('./core/wallet/getAddressBalance');
const sendToAddress = require('./core/wallet/sendToAddress');
const registerMasternode = require('./core/wallet/registerMasternode');

const createClientWithFundedWallet = require('./sdk/createClientWithFundedWallet');

const generateToAddressTaskFactory = require('./listr/tasks/wallet/generateToAddressTaskFactory');
const registerMasternodeTaskFactory = require('./listr/tasks/registerMasternodeTaskFactory');
const initTaskFactory = require('./listr/tasks/platform/initTaskFactory');
const startNodeTaskFactory = require('./listr/tasks/startNodeTaskFactory');

async function createDIContainer() {
  const container = createAwilixContainer({
    injectionMode: InjectionMode.CLASSIC,
  });

  /**
   * Docker
   */
  container.register({
    docker: asFunction(() => (
      new Docker()
    )).singleton(),
    dockerCompose: asClass(DockerCompose),
    startedContainers: asFunction(() => (
      new StartedContainers()
    )).singleton(),
    stopAllContainers: asFunction(stopAllContainersFactory).singleton(),
  });

  /**
   * Core
   */
  container.register({
    createRpcClient: asValue(createRpcClient),
    waitForCoreStart: asValue(waitForCoreStart),
    waitForCoreSync: asValue(waitForCoreSync),
    startCore: asFunction(startCoreFactory).singleton(),
    waitForBlocks: asValue(waitForBlocks),
    waitForConfirmations: asValue(waitForConfirmations),
    generateBlsKeys: asValue(generateBlsKeys),
  });

  /**
   * Core Wallet
   */
  container.register({
    createNewAddress: asValue(createNewAddress),
    generateBlocks: asValue(generateBlocks),
    generateToAddress: asValue(generateToAddress),
    importPrivateKey: asValue(importPrivateKey),
    getAddressBalance: asValue(getAddressBalance),
    sendToAddress: asValue(sendToAddress),
    registerMasternode: asValue(registerMasternode),
  });

  /**
   * Dash SDK
   */
  container.register({
    createClientWithFundedWallet: asValue(createClientWithFundedWallet),
  });

  /**
   * Tasks
   */
  container.register({
    generateToAddressTask: asFunction(generateToAddressTaskFactory),
    registerMasternodeTask: asFunction(registerMasternodeTaskFactory),
    initTask: asFunction(initTaskFactory),
    startNodeTask: asFunction(startNodeTaskFactory),
  });

  return container;
}

module.exports = createDIContainer;
