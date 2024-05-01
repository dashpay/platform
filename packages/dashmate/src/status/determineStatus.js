import { DockerStatusEnum } from './enums/dockerStatus.js';
import { MasternodeSyncAssetEnum } from './enums/masternodeSyncAsset.js';
import { ServiceStatusEnum } from './enums/serviceStatus.js';

export default {
  /**
   * Determine status based on the docker compose output
   * @param dockerCompose {DockerCompose}
   * @param {Config} config
   * @param serviceName {string}
   * @return {DockerStatusEnum}
   */
  docker: async (dockerCompose, config, serviceName) => {
    const containerInfo = await dockerCompose.inspectService(
      config,
      serviceName,
    );

    return containerInfo.State.Status;
  },
  /**
   * Determine ServiceStatus based on DockerStatusEnum and MasternodeSyncAssetEnum
   * @param dockerStatus {DockerStatusEnum}
   * @param syncAsset {MasternodeSyncAssetEnum}
   * @returns {ServiceStatusEnum}
   */
  core: (dockerStatus, syncAsset) => {
    if (syncAsset && dockerStatus === DockerStatusEnum.running) {
      if (syncAsset === MasternodeSyncAssetEnum.MASTERNODE_SYNC_FINISHED) {
        return ServiceStatusEnum.up;
      }
      return ServiceStatusEnum.syncing;
    }
    return ServiceStatusEnum.error;
  },
  /**
   * Determine platform ServiceStatus based on DockerStatusEnum and core readiness
   * @param dockerStatus {DockerStatusEnum}
   * @param coreIsSynced {boolean}
   * @returns {ServiceStatusEnum}
   */
  platform: (dockerStatus, coreIsSynced) => {
    if (dockerStatus === DockerStatusEnum.running) {
      return coreIsSynced ? ServiceStatusEnum.up : ServiceStatusEnum.wait_for_core;
    }

    return ServiceStatusEnum.error;
  },
};
