import {
  createContainer as createAwilixContainer,
  InjectionMode,
  asFunction,
  asValue,
  asClass,
} from 'awilix';

import Docker from 'dockerode';

import getServiceListFactory from './docker/getServiceListFactory.js';
import ensureFileMountExistsFactory from './docker/ensureFileMountExistsFactory.js';
import getConnectionHostFactory from './docker/getConnectionHostFactory.js';
import ConfigFileJsonRepository from './config/configFile/ConfigFileJsonRepository.js';
import createConfigFileFactory from './config/configFile/createConfigFileFactory.js';
import migrateConfigFileFactory from './config/configFile/migrateConfigFileFactory.js';
import DefaultConfigs from './config/DefaultConfigs.js';

import renderTemplateFactory from './templates/renderTemplateFactory.js';
import renderServiceTemplatesFactory from './templates/renderServiceTemplatesFactory.js';
import writeServiceConfigsFactory from './templates/writeServiceConfigsFactory.js';

import DockerCompose from './docker/DockerCompose.js';
import StartedContainers from './docker/StartedContainers.js';
import stopAllContainersFactory from './docker/stopAllContainersFactory.js';
import dockerPullFactory from './docker/dockerPullFactory.js';
import resolveDockerHostIpFactory from './docker/resolveDockerHostIpFactory.js';

import startCoreFactory from './core/startCoreFactory.js';
import createRpcClient from './core/createRpcClient.js';
import waitForCoreStart from './core/waitForCoreStart.js';
import waitForCoreSync from './core/waitForCoreSync.js';
import waitForMasternodesSync from './core/waitForMasternodesSync.js';
import waitForBlocks from './core/waitForBlocks.js';
import waitForConfirmations from './core/waitForConfirmations.js';
import generateBlsKeys from './core/generateBlsKeys.js';
import activateCoreSpork from './core/activateCoreSpork.js';
import waitForCorePeersConnected from './core/waitForCorePeersConnected.js';

import createNewAddress from './core/wallet/createNewAddress.js';
import generateBlocks from './core/wallet/generateBlocks.js';
import generateToAddress from './core/wallet/generateToAddress.js';
import importPrivateKey from './core/wallet/importPrivateKey.js';
import getAddressBalance from './core/wallet/getAddressBalance.js';
import sendToAddress from './core/wallet/sendToAddress.js';
import registerMasternode from './core/wallet/registerMasternode.js';
import waitForBalanceToConfirm from './core/wallet/waitForBalanceToConfirm.js';

import getCoreScopeFactory from './status/scopes/core.js';
import getMasternodeScopeFactory from './status/scopes/masternode.js';
import getPlatformScopeFactory from './status/scopes/platform.js';
import getOverviewScopeFactory from './status/scopes/overview.js';
import getServicesScopeFactory from './status/scopes/services.js';
import getHostScopeFactory from './status/scopes/host.js';

import generateToAddressTaskFactory from './listr/tasks/wallet/generateToAddressTaskFactory.js';
import registerMasternodeTaskFactory from './listr/tasks/registerMasternodeTaskFactory.js';
import startNodeTaskFactory from './listr/tasks/startNodeTaskFactory.js';

import createTenderdashRpcClient from './tenderdash/createTenderdashRpcClient.js';
import setupLocalPresetTaskFactory from './listr/tasks/setup/setupLocalPresetTaskFactory.js';
import setupRegularPresetTaskFactory from './listr/tasks/setup/setupRegularPresetTaskFactory.js';
import stopNodeTaskFactory from './listr/tasks/stopNodeTaskFactory.js';
import restartNodeTaskFactory from './listr/tasks/restartNodeTaskFactory.js';
import resetNodeTaskFactory from './listr/tasks/resetNodeTaskFactory.js';
import configureCoreTaskFactory from './listr/tasks/setup/local/configureCoreTaskFactory.js';
import configureTenderdashTaskFactory from './listr/tasks/setup/local/configureTenderdashTaskFactory.js';
import obtainSelfSignedCertificateTaskFactory from './listr/tasks/ssl/selfSigned/obtainSelfSignedCertificateTaskFactory.js';
import waitForNodeToBeReadyTaskFactory from './listr/tasks/platform/waitForNodeToBeReadyTaskFactory.js';
import enableCoreQuorumsTaskFactory from './listr/tasks/setup/local/enableCoreQuorumsTaskFactory.js';
import startGroupNodesTaskFactory from './listr/tasks/startGroupNodesTaskFactory.js';
import buildServicesTaskFactory from './listr/tasks/buildServicesTaskFactory.js';
import reindexNodeTaskFactory from './listr/tasks/reindexNodeTaskFactory.js';

import updateNodeFactory from './update/updateNodeFactory.js';

import generateHDPrivateKeys from './util/generateHDPrivateKeys.js';

import obtainZeroSSLCertificateTaskFactory from './listr/tasks/ssl/zerossl/obtainZeroSSLCertificateTaskFactory.js';
import VerificationServer from './listr/tasks/ssl/VerificationServer.js';
import saveCertificateTaskFactory from './listr/tasks/ssl/saveCertificateTask.js';

import createZeroSSLCertificate from './ssl/zerossl/createZeroSSLCertificate.js';
import verifyDomain from './ssl/zerossl/verifyDomain.js';
import downloadCertificate from './ssl/zerossl/downloadCertificate.js';
import getCertificate from './ssl/zerossl/getCertificate.js';
import listCertificates from './ssl/zerossl/listCertificates.js';
import generateCsr from './ssl/zerossl/generateCsr.js';
import generateKeyPair from './ssl/generateKeyPair.js';
import createSelfSignedCertificate from './ssl/selfSigned/createSelfSignedCertificate.js';

import scheduleRenewZeroSslCertificateFactory from './helper/scheduleRenewZeroSslCertificateFactory.js';
import registerMasternodeGuideTaskFactory from './listr/tasks/setup/regular/registerMasternodeGuideTaskFactory.js';
import configureNodeTaskFactory from './listr/tasks/setup/regular/configureNodeTaskFactory.js';
import configureSSLCertificateTaskFactory from './listr/tasks/setup/regular/configureSSLCertificateTaskFactory.js';
import createHttpApiServerFactory from './helper/api/createHttpApiServerFactory.js';
import resolveDockerSocketPath from './docker/resolveDockerSocketPath.js';
import HomeDir from './config/HomeDir.js';
import getBaseConfigFactory from '../configs/defaults/getBaseConfigFactory.js';
import getLocalConfigFactory from '../configs/defaults/getLocalConfigFactory.js';
import getTestnetConfigFactory from '../configs/defaults/getTestnetConfigFactory.js';
import getMainnetConfigFactory from '../configs/defaults/getMainnetConfigFactory.js';
import getConfigFileMigrationsFactory from '../configs/getConfigFileMigrationsFactory.js';
import assertLocalServicesRunningFactory from './test/asserts/assertLocalServicesRunningFactory.js';
import assertServiceRunningFactory from './test/asserts/assertServiceRunningFactory.js';
import generateEnvsFactory from './config/generateEnvsFactory.js';
import getConfigProfilesFactory from './config/getConfigProfilesFactory.js';
import createIpAndPortsFormFactory from './listr/prompts/createIpAndPortsForm.js';
import registerMasternodeWithCoreWalletFactory from './listr/tasks/setup/regular/registerMasternode/registerMasternodeWithCoreWallet.js';
import registerMasternodeWithDMTFactory from './listr/tasks/setup/regular/registerMasternode/registerMasternodeWithDMT.js';
import writeConfigTemplatesFactory from './templates/writeConfigTemplatesFactory.js';
import importCoreDataTaskFactory from './listr/tasks/setup/regular/importCoreDataTaskFactory.js';
import verifySystemRequirementsTaskFactory
  from './listr/tasks/setup/regular/verifySystemRequirementsTaskFactory.js';

/**
 * @param {Object} [options]
 * @returns {Promise<AwilixContainer<any>>}
 */
export default async function createDIContainer(options = {}) {
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
    importCoreDataTask: asFunction(importCoreDataTaskFactory).singleton(),
    verifySystemRequirementsTask: asFunction(verifySystemRequirementsTaskFactory)
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
