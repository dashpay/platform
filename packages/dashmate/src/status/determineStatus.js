const ContainerIsNotPresentError = require('../docker/errors/ContainerIsNotPresentError');
const MasternodeSyncAssetEnum = require('../enums/masternodeSyncAsset');
const ServiceStatusEnum = require('../enums/serviceStatus');
const CoreService = require('../core/CoreService');
const createRpcClient = require('../core/createRpcClient');

/**
 * Determine status based on the docker compose output
 * @param dockerCompose {DockerCompose}
 * @param config {Config}
 * @param serviceName {string}
 */
const determineStatus = async (dockerCompose, config, serviceName) => {
  try {
    const containerInfo = await dockerCompose.inspectService(config.toEnvs(), serviceName);

    const status = containerInfo.State.Status;

    if (status === ServiceStatusEnum.running && serviceName === 'core') {
      const coreService = new CoreService(
        config,
        createRpcClient(
          {
            port: config.get('core.rpc.port'),
            user: config.get('core.rpc.user'),
            pass: config.get('core.rpc.password'),
          },
        ),
        dockerCompose.docker.getContainer('core'),
      );

      const masternodeSyncResult = await coreService.getRpcClient().mnsync('status');
      const { AssetName: syncAsset } = masternodeSyncResult.result;

      return syncAsset === MasternodeSyncAssetEnum.MASTERNODE_SYNC_FINISHED ? status : ServiceStatusEnum.syncing;
    }

    return status;
  } catch (e) {
    if (e instanceof ContainerIsNotPresentError) {
      return 'not_started';
    }
    throw e;
  }
};

module.exports = determineStatus;
