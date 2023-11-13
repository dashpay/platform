import {
  createContainer as createAwilixContainer,
  InjectionMode,
  asFunction,
  asValue,
  asClass,
} from 'awilix';

import Docker from 'dockerode';

import {getServiceListFactory} from './docker/getServiceListFactory';
import {ensureFileMountExistsFactory} from './docker/ensureFileMountExistsFactory';
import {getConnectionHostFactory} from './docker/getConnectionHostFactory';
import {ConfigFileJsonRepository} from './config/configFile/ConfigFileJsonRepository';
import {createConfigFileFactory} from './config/configFile/createConfigFileFactory';
import {migrateConfigFileFactory} from './config/configFile/migrateConfigFileFactory';
import {DefaultConfigs} from './config/DefaultConfigs';

import {renderTemplateFactory} from './templates/renderTemplateFactory';
import {renderServiceTemplatesFactory} from './templates/renderServiceTemplatesFactory';
import {writeServiceConfigsFactory} from './templates/writeServiceConfigsFactory';

import {DockerCompose} from './docker/DockerCompose';
import {StartedContainers} from './docker/StartedContainers';
import {stopAllContainersFactory} from './docker/stopAllContainersFactory';
import {dockerPullFactory} from './docker/dockerPullFactory';
import {resolveDockerHostIpFactory} from './docker/resolveDockerHostIpFactory';

import {startCoreFactory} from './core/startCoreFactory';
import {createRpcClient} from './core/createRpcClient';
import {waitForCoreStart} from './core/waitForCoreStart';
import {waitForCoreSync} from './core/waitForCoreSync';
import {waitForMasternodesSync} from './core/waitForMasternodesSync';
import {waitForBlocks} from './core/waitForBlocks';
import {waitForConfirmations} from './core/waitForConfirmations'
import {generateBlsKeys} from './core/generateBlsKeys'
import {activateCoreSpork} from './core/activateCoreSpork'
import {waitForCorePeersConnected} from './core/waitForCorePeersConnected'

import {createNewAddress} from './core/wallet/createNewAddress';
import {generateBlocks} from './core/wallet/generateBlocks';
import {generateToAddress} from './core/wallet/generateToAddress';
import {importPrivateKey} from './core/wallet/importPrivateKey';
import {getAddressBalance} from './core/wallet/getAddressBalance';
import {sendToAddress} from './core/wallet/sendToAddress';
import {registerMasternode} from './core/wallet/registerMasternode';
import {waitForBalanceToConfirm} from './core/wallet/waitForBalanceToConfirm';

import {getCoreScopeFactory} from './status/scopes/core';
import {getMasternodeScopeFactory} from './status/scopes/masternode';
import {getPlatformScopeFactory} from './status/scopes/platform';
import {getOverviewScopeFactory} from './status/scopes/overview';
import {getServicesScopeFactory} from './status/scopes/services';
import {getHostScopeFactory} from './status/scopes/host';

import {generateToAddressTaskFactory} from './listr/tasks/wallet/generateToAddressTaskFactory';
import {registerMasternodeTaskFactory} from './listr/tasks/registerMasternodeTaskFactory';
import {featureFlagTaskFactory} from './listr/tasks/platform/featureFlagTaskFactory';
import {startNodeTaskFactory} from './listr/tasks/startNodeTaskFactory';

import {createTenderdashRpcClient} from './tenderdash/createTenderdashRpcClient';
import {
  setupLocalPresetTaskFactory
} from './listr/tasks/setup/setupLocalPresetTaskFactory';
import {
  setupRegularPresetTaskFactory
} from './listr/tasks/setup/setupRegularPresetTaskFactory';
import {stopNodeTaskFactory} from './listr/tasks/stopNodeTaskFactory';
import {restartNodeTaskFactory} from './listr/tasks/restartNodeTaskFactory';
import {resetNodeTaskFactory} from './listr/tasks/resetNodeTaskFactory';
import {configureCoreTaskFactory} from './listr/tasks/setup/local/configureCoreTaskFactory';
import {
  configureTenderdashTaskFactory
} from './listr/tasks/setup/local/configureTenderdashTaskFactory';
import {
  obtainSelfSignedCertificateTaskFactory
} from './listr/tasks/ssl/selfSigned/obtainSelfSignedCertificateTaskFactory';
import {
  waitForNodeToBeReadyTaskFactory
} from './listr/tasks/platform/waitForNodeToBeReadyTaskFactory';
import {
  enableCoreQuorumsTaskFactory
} from './listr/tasks/setup/local/enableCoreQuorumsTaskFactory';
import {startGroupNodesTaskFactory} from './listr/tasks/startGroupNodesTaskFactory';
import {buildServicesTaskFactory} from './listr/tasks/buildServicesTaskFactory';
import {reindexNodeTaskFactory} from './listr/tasks/reindexNodeTaskFactory';

import {updateNodeFactory} from './update/updateNodeFactory';

import {generateHDPrivateKeys} from './util/generateHDPrivateKeys';

import {
  obtainZeroSSLCertificateTaskFactory
} from './listr/tasks/ssl/zerossl/obtainZeroSSLCertificateTaskFactory';
import {VerificationServer} from './listr/tasks/ssl/VerificationServer';
import {saveCertificateTaskFactory} from './listr/tasks/ssl/saveCertificateTask';

import {createZeroSSLCertificate} from './ssl/zerossl/createZeroSSLCertificate';
import {verifyDomain} from './ssl/zerossl/verifyDomain';
import {downloadCertificate} from './ssl/zerossl/downloadCertificate';
import {getCertificate} from './ssl/zerossl/getCertificate';
import {listCertificates} from './ssl/zerossl/listCertificates';
import {generateCsr} from './ssl/zerossl/generateCsr';
import {generateKeyPair} from './ssl/generateKeyPair';
import {createSelfSignedCertificate} from './ssl/selfSigned/createSelfSignedCertificate';

import {
  scheduleRenewZeroSslCertificateFactory
} from './helper/scheduleRenewZeroSslCertificateFactory';
import {
  registerMasternodeGuideTaskFactory
} from './listr/tasks/setup/regular/registerMasternodeGuideTaskFactory';
import {configureNodeTaskFactory} from './listr/tasks/setup/regular/configureNodeTaskFactory';
import {
  configureSSLCertificateTaskFactory
} from './listr/tasks/setup/regular/configureSSLCertificateTaskFactory';
import {createHttpApiServerFactory} from './helper/api/createHttpApiServerFactory';
import {resolveDockerSocketPath} from './docker/resolveDockerSocketPath';
import HomeDir from './config/HomeDir';
import {getBaseConfigFactory} from '../configs/defaults/getBaseConfigFactory';
import {getLocalConfigFactory} from '../configs/defaults/getLocalConfigFactory';
import {getTestnetConfigFactory} from '../configs/defaults/getTestnetConfigFactory';
import {getMainnetConfigFactory} from '../configs/defaults/getMainnetConfigFactory';
import {
  getConfigFileMigrationsFactory
} from '../configs/getConfigFileMigrationsFactory';
import {
  assertLocalServicesRunningFactory
} from './test/asserts/assertLocalServicesRunningFactory';
import {assertServiceRunningFactory} from './test/asserts/assertServiceRunningFactory';
import {generateEnvsFactory} from './config/generateEnvsFactory';
import {getConfigProfilesFactory} from './config/getConfigProfilesFactory';
import {createIpAndPortsFormFactory} from './listr/prompts/createIpAndPortsForm';
import {
  registerMasternodeWithCoreWalletFactory
} from './listr/tasks/setup/regular/registerMasternode/registerMasternodeWithCoreWallet';
import {
  registerMasternodeWithDMTFactory
} from './listr/tasks/setup/regular/registerMasternode/registerMasternodeWithDMT';
import {writeConfigTemplatesFactory} from './templates/writeConfigTemplatesFactory';

/**
 * @param {Object} [options]
 * @returns {Promise<AwilixContainer<any>>}
 */
export async function createDIContainer(options = {}) {
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
  });

  /**
   * Templates
   */
  container.register({
    renderTemplate: asFunction(renderTemplateFactory).singleton(),
    renderServiceTemplates: asFunction(renderServiceTemplatesFactory).singleton(),
    writeServiceConfigs: asFunction(writeServiceConfigsFactory).singleton(),
    writeConfigTemplates: asFunction(writeConfigTemplatesFactory).singleton(),
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
