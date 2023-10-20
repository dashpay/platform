const {
  createContainer: createAwilixContainer,
  InjectionMode,
  asFunction,
  asValue,
  asClass,
} = require('awilix');

const Docker = require('dockerode');

const getServiceListFactory = require('./docker/getServiceListFactory');
const ensureFileMountExistsFactory = require('./docker/ensureFileMountExistsFactory');
const getConnectionHostFactory = require('./docker/getConnectionHostFactory');
const ConfigFileJsonRepository = require('./config/configFile/ConfigFileJsonRepository');
const createConfigFileFactory = require('./config/configFile/createConfigFileFactory');
const migrateConfigFileFactory = require('./config/configFile/migrateConfigFileFactory');
const DefaultConfigs = require('./config/DefaultConfigs');

const renderTemplateFactory = require('./templates/renderTemplateFactory');
const renderServiceTemplatesFactory = require('./templates/renderServiceTemplatesFactory');
const writeServiceConfigsFactory = require('./templates/writeServiceConfigsFactory');

const DockerCompose = require('./docker/DockerCompose');
const StartedContainers = require('./docker/StartedContainers');
const stopAllContainersFactory = require('./docker/stopAllContainersFactory');
const dockerPullFactory = require('./docker/dockerPullFactory');
const resolveDockerHostIpFactory = require('./docker/resolveDockerHostIpFactory');

const startCoreFactory = require('./core/startCoreFactory');
const createRpcClient = require('./core/createRpcClient');
const waitForCoreStart = require('./core/waitForCoreStart');
const waitForCoreSync = require('./core/waitForCoreSync');
const waitForMasternodesSync = require('./core/waitForMasternodesSync');
const waitForBlocks = require('./core/waitForBlocks');
const waitForConfirmations = require('./core/waitForConfirmations');
const generateBlsKeys = require('./core/generateBlsKeys');
const activateCoreSpork = require('./core/activateCoreSpork');
const waitForCorePeersConnected = require('./core/waitForCorePeersConnected');

const createNewAddress = require('./core/wallet/createNewAddress');
const generateBlocks = require('./core/wallet/generateBlocks');
const generateToAddress = require('./core/wallet/generateToAddress');
const importPrivateKey = require('./core/wallet/importPrivateKey');
const getAddressBalance = require('./core/wallet/getAddressBalance');
const sendToAddress = require('./core/wallet/sendToAddress');
const registerMasternode = require('./core/wallet/registerMasternode');
const waitForBalanceToConfirm = require('./core/wallet/waitForBalanceToConfirm');

const getCoreScopeFactory = require('./status/scopes/core');
const getMasternodeScopeFactory = require('./status/scopes/masternode');
const getPlatformScopeFactory = require('./status/scopes/platform');
const getOverviewScopeFactory = require('./status/scopes/overview');
const getServicesScopeFactory = require('./status/scopes/services');
const getHostScopeFactory = require('./status/scopes/host');

const generateToAddressTaskFactory = require('./listr/tasks/wallet/generateToAddressTaskFactory');
const registerMasternodeTaskFactory = require('./listr/tasks/registerMasternodeTaskFactory');
const featureFlagTaskFactory = require('./listr/tasks/platform/featureFlagTaskFactory');
const startNodeTaskFactory = require('./listr/tasks/startNodeTaskFactory');

const createTenderdashRpcClient = require('./tenderdash/createTenderdashRpcClient');
const setupLocalPresetTaskFactory = require('./listr/tasks/setup/setupLocalPresetTaskFactory');
const setupRegularPresetTaskFactory = require('./listr/tasks/setup/setupRegularPresetTaskFactory');
const stopNodeTaskFactory = require('./listr/tasks/stopNodeTaskFactory');
const restartNodeTaskFactory = require('./listr/tasks/restartNodeTaskFactory');
const resetNodeTaskFactory = require('./listr/tasks/resetNodeTaskFactory');
const configureCoreTaskFactory = require('./listr/tasks/setup/local/configureCoreTaskFactory');
const configureTenderdashTaskFactory = require('./listr/tasks/setup/local/configureTenderdashTaskFactory');
const obtainSelfSignedCertificateTaskFactory = require('./listr/tasks/ssl/selfSigned/obtainSelfSignedCertificateTaskFactory');
const waitForNodeToBeReadyTaskFactory = require('./listr/tasks/platform/waitForNodeToBeReadyTaskFactory');
const enableCoreQuorumsTaskFactory = require('./listr/tasks/setup/local/enableCoreQuorumsTaskFactory');
const startGroupNodesTaskFactory = require('./listr/tasks/startGroupNodesTaskFactory');
const buildServicesTaskFactory = require('./listr/tasks/buildServicesTaskFactory');
const reindexNodeTaskFactory = require('./listr/tasks/reindexNodeTaskFactory');

const updateNodeFactory = require('./update/updateNodeFactory');

const generateHDPrivateKeys = require('./util/generateHDPrivateKeys');
const resolvePublicIpV4 = require('./util/resolvePublicIpV4');

const obtainZeroSSLCertificateTaskFactory = require('./listr/tasks/ssl/zerossl/obtainZeroSSLCertificateTaskFactory');
const VerificationServer = require('./listr/tasks/ssl/VerificationServer');
const saveCertificateTaskFactory = require('./listr/tasks/ssl/saveCertificateTask');

const createZeroSSLCertificate = require('./ssl/zerossl/createZeroSSLCertificate');
const verifyDomain = require('./ssl/zerossl/verifyDomain');
const downloadCertificate = require('./ssl/zerossl/downloadCertificate');
const getCertificate = require('./ssl/zerossl/getCertificate');
const listCertificates = require('./ssl/zerossl/listCertificates');
const generateCsr = require('./ssl/zerossl/generateCsr');
const generateKeyPair = require('./ssl/generateKeyPair');
const createSelfSignedCertificate = require('./ssl/selfSigned/createSelfSignedCertificate');

const scheduleRenewZeroSslCertificateFactory = require('./helper/scheduleRenewZeroSslCertificateFactory');
const registerMasternodeGuideTaskFactory = require('./listr/tasks/setup/regular/registerMasternodeGuideTaskFactory');
const configureNodeTaskFactory = require('./listr/tasks/setup/regular/configureNodeTaskFactory');
const configureSSLCertificateTaskFactory = require('./listr/tasks/setup/regular/configureSSLCertificateTaskFactory');
const createHttpApiServerFactory = require('./helper/api/createHttpApiServerFactory');
const resolveDockerSocketPath = require('./docker/resolveDockerSocketPath');
const HomeDir = require('./config/HomeDir');
const getBaseConfigFactory = require('../configs/defaults/getBaseConfigFactory');
const getLocalConfigFactory = require('../configs/defaults/getLocalConfigFactory');
const getTestnetConfigFactory = require('../configs/defaults/getTestnetConfigFactory');
const getMainnetConfigFactory = require('../configs/defaults/getMainnetConfigFactory');
const getConfigFileMigrationsFactory = require('../configs/getConfigFileMigrationsFactory');
const assertLocalServicesRunningFactory = require('./test/asserts/assertLocalServicesRunningFactory');
const assertServiceRunningFactory = require('./test/asserts/assertServiceRunningFactory');
const generateEnvsFactory = require('./config/generateEnvsFactory');
const getConfigProfilesFactory = require('./config/getConfigProfilesFactory');
const createIpAndPortsFormFactory = require('./listr/prompts/createIpAndPortsForm');
const createPortIsNotReachableFormFactory = require('./listr/prompts/createPortIsNotReachableForm');
const registerMasternodeWithCoreWalletFactory = require('./listr/tasks/setup/regular/registerMasternode/registerMasternodeWithCoreWallet');
const registerMasternodeWithDMTFactory = require('./listr/tasks/setup/regular/registerMasternode/registerMasternodeWithDMT');

/**
 * @param {Object} [options]
 * @returns {Promise<AwilixContainer<any>>}
 */
async function createDIContainer(options = {}) {
  const container = createAwilixContainer({
    injectionMode: InjectionMode.CLASSIC,
  });

  /**
   * Config
   */
  container.register({
    // TODO: It creates a directory on the disk when we create DI container. Doesn't smell good
    homeDir: asFunction(() => (
      HomeDir.createWithPathOrDefault(options.DASHMATE_HOME_DIR)
    )).singleton(),
    getServiceList: asFunction(getServiceListFactory).singleton(),
    configFileRepository: asClass(ConfigFileJsonRepository).singleton(),
    getBaseConfig: asFunction(getBaseConfigFactory).singleton(),
    getLocalConfig: asFunction(getLocalConfigFactory).singleton(),
    getTestnetConfig: asFunction(getTestnetConfigFactory).singleton(),
    getMainnetConfig: asFunction(getMainnetConfigFactory).singleton(),
    defaultConfigs: asFunction((
      getBaseConfig,
      getLocalConfig,
      getTestnetConfig,
      getMainnetConfig,
    ) => new DefaultConfigs([
      getBaseConfig,
      getLocalConfig,
      getTestnetConfig,
      getMainnetConfig,
    ])).singleton(),
    createConfigFile: asFunction(createConfigFileFactory).singleton(),
    getConfigFileMigrations: asFunction(getConfigFileMigrationsFactory).singleton(),
    migrateConfigFile: asFunction(migrateConfigFileFactory).singleton(),
    isHelper: asValue(process.env.DASHMATE_HELPER === '1'),
    getConnectionHost: asFunction(getConnectionHostFactory).singleton(),
    generateEnvs: asFunction(generateEnvsFactory).singleton(),
    getConfigProfiles: asFunction(getConfigProfilesFactory).singleton(),
    ensureFileMountExists: asFunction(ensureFileMountExistsFactory).singleton(),
    // `configFile` and `config` are registering on command init
  });

  /**
   * Update
   */
  container.register({
    updateNode: asFunction(updateNodeFactory).singleton(),
  });

  /**
   * Utils
   */
  container.register({
    generateHDPrivateKeys: asValue(generateHDPrivateKeys),
    resolvePublicIpV4: asValue(resolvePublicIpV4),
  });

  /**
   * Templates
   */
  container.register({
    renderTemplate: asFunction(renderTemplateFactory).singleton(),
    renderServiceTemplates: asFunction(renderServiceTemplatesFactory).singleton(),
    writeServiceConfigs: asFunction(writeServiceConfigsFactory).singleton(),
  });

  /**
   * SSL
   */
  container.register({
    createZeroSSLCertificate: asValue(createZeroSSLCertificate),
    generateCsr: asValue(generateCsr),
    generateKeyPair: asValue(generateKeyPair),
    verifyDomain: asValue(verifyDomain),
    downloadCertificate: asValue(downloadCertificate),
    getCertificate: asValue(getCertificate),
    listCertificates: asValue(listCertificates),
    createSelfSignedCertificate: asValue(createSelfSignedCertificate),
    verificationServer: asClass(VerificationServer).singleton(),
  });

  /**
   * Docker
   */

  const dockerOptions = {};
  try {
    dockerOptions.socketPath = await resolveDockerSocketPath();
  } catch (e) {
    // Here we skip possible error which is happening if docker is not installed or not running
    // It will be handled in the logic below
  }

  container.register({
    docker: asFunction(() => (
      new Docker(dockerOptions)
    )).singleton(),
    dockerCompose: asClass(DockerCompose).singleton(),
    startedContainers: asFunction(() => (
      new StartedContainers()
    )).singleton(),
    stopAllContainers: asFunction(stopAllContainersFactory).singleton(),
    dockerPull: asFunction(dockerPullFactory).singleton(),
    resolveDockerHostIp: asFunction(resolveDockerHostIpFactory).singleton(),
  });

  /**
   * Core
   */
  container.register({
    createRpcClient: asValue(createRpcClient),
    waitForCoreStart: asValue(waitForCoreStart),
    waitForCoreSync: asValue(waitForCoreSync),
    waitForMasternodesSync: asValue(waitForMasternodesSync),
    startCore: asFunction(startCoreFactory).singleton(),
    waitForBlocks: asValue(waitForBlocks),
    waitForConfirmations: asValue(waitForConfirmations),
    generateBlsKeys: asValue(generateBlsKeys),
    activateCoreSpork: asValue(activateCoreSpork),
    waitForCorePeersConnected: asValue(waitForCorePeersConnected),
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
    waitForBalanceToConfirm: asValue(waitForBalanceToConfirm),
  });

  /**
   * Tenderdash
   */
  container.register({
    createTenderdashRpcClient: asValue(createTenderdashRpcClient),
  });

  /**
   * Prompts
   */
  container.register({
    createIpAndPortsForm: asFunction(createIpAndPortsFormFactory).singleton(),
    createPortIsNotReachableForm: asFunction(createPortIsNotReachableFormFactory).singleton(),
  });

  /**
   * Tasks
   */
  container.register({
    buildServicesTask: asFunction(buildServicesTaskFactory).singleton(),
    startGroupNodesTask: asFunction(startGroupNodesTaskFactory).singleton(),
    generateToAddressTask: asFunction(generateToAddressTaskFactory).singleton(),
    registerMasternodeTask: asFunction(registerMasternodeTaskFactory).singleton(),
    featureFlagTask: asFunction(featureFlagTaskFactory).singleton(),
    startNodeTask: asFunction(startNodeTaskFactory).singleton(),
    stopNodeTask: asFunction(stopNodeTaskFactory).singleton(),
    restartNodeTask: asFunction(restartNodeTaskFactory).singleton(),
    resetNodeTask: asFunction(resetNodeTaskFactory).singleton(),
    setupLocalPresetTask: asFunction(setupLocalPresetTaskFactory).singleton(),
    setupRegularPresetTask: asFunction(setupRegularPresetTaskFactory).singleton(),
    configureCoreTask: asFunction(configureCoreTaskFactory).singleton(),
    configureTenderdashTask: asFunction(configureTenderdashTaskFactory).singleton(),
    waitForNodeToBeReadyTask: asFunction(waitForNodeToBeReadyTaskFactory).singleton(),
    enableCoreQuorumsTask: asFunction(enableCoreQuorumsTaskFactory).singleton(),
    registerMasternodeGuideTask: asFunction(registerMasternodeGuideTaskFactory).singleton(),
    obtainZeroSSLCertificateTask: asFunction(obtainZeroSSLCertificateTaskFactory).singleton(),
    obtainSelfSignedCertificateTask: asFunction(obtainSelfSignedCertificateTaskFactory).singleton(),
    saveCertificateTask: asFunction(saveCertificateTaskFactory),
    reindexNodeTask: asFunction(reindexNodeTaskFactory).singleton(),
    getCoreScope: asFunction(getCoreScopeFactory).singleton(),
    getMasternodeScope: asFunction(getMasternodeScopeFactory).singleton(),
    getPlatformScope: asFunction(getPlatformScopeFactory).singleton(),
    getOverviewScope: asFunction(getOverviewScopeFactory).singleton(),
    getServicesScope: asFunction(getServicesScopeFactory).singleton(),
    getHostScope: asFunction(getHostScopeFactory).singleton(),
    configureNodeTask: asFunction(configureNodeTaskFactory).singleton(),
    configureSSLCertificateTask: asFunction(configureSSLCertificateTaskFactory).singleton(),
    registerMasternodeWithCoreWallet: asFunction(registerMasternodeWithCoreWalletFactory)
      .singleton(),
    registerMasternodeWithDMT: asFunction(registerMasternodeWithDMTFactory)
      .singleton(),
  });

  /**
   * Helper
   */
  container.register({
    scheduleRenewZeroSslCertificate: asFunction(scheduleRenewZeroSslCertificateFactory).singleton(),
    createHttpApiServer: asFunction(createHttpApiServerFactory).singleton(),
  });

  /**
   * Tests
   */
  container.register({
    assertLocalServicesRunning: asFunction(assertLocalServicesRunningFactory).singleton(),
    assertServiceRunning: asFunction(assertServiceRunningFactory).singleton(),
  });

  return container;
}

module.exports = createDIContainer;
