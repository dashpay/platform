const DockerStatusEnum = require("../enums/dockerStatus");
const ServiceStatusEnum = require("../enums/serviceStatus");
const MasternodeSyncAssetEnum = require("../enums/masternodeSyncAsset");

module.exports = {
  /**
   * Determine status based on the docker compose output
   * @param dockerCompose {DockerCompose}
   * @param config {Config}
   * @param serviceName {string}
   * @return {DockerStatusEnum}
   */
  docker: async (dockerCompose, config, serviceName) => {
    try {
      const containerInfo = await dockerCompose.inspectService(config.toEnvs(), serviceName);

      return containerInfo.State.Status;
    } catch (e) {
      throw e;
    }
  },
  /**
   * Determine ServiceStatus based on DockerStatusEnum and MasternodeSyncAssetEnum
   * @param dockerStatus {DockerStatusEnum}
   * @param syncAsset {MasternodeSyncAssetEnum}
   * @returns {ServiceStatusEnum}
   */
  core: (dockerStatus, syncAsset) => {
    if (dockerStatus === DockerStatusEnum.running) {
      if (syncAsset === MasternodeSyncAssetEnum.MASTERNODE_SYNC_FINISHED) {
        return ServiceStatusEnum.up
      } else {
        return ServiceStatusEnum.syncing
      }
    } else {
      return ServiceStatusEnum.error
    }
  }
};
