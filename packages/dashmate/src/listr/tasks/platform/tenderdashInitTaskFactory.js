const { Listr } = require('listr2');

/**
 * @param {initializeTenderdashNode} initializeTenderdashNode
 * @param {Docker} docker
 * @return {tenderdashInitTask}
 */
function tenderdashInitTaskFactory(
  initializeTenderdashNode,
  docker,
) {
  /**
   * @typedef {tenderdashInitTask}
   * @param {Config} config
   * @return {Listr}
   */
  function tenderdashInitTask(
    config,
  ) {
    return new Listr([
      {
        title: 'Generate node keys and data',
        task: async (ctx, task) => {
          const isValidatorKeyPresent = Object.keys(config.get('platform.drive.tenderdash.validatorKey')).length !== 0;
          const isNodeKeyPresent = Object.keys(config.get('platform.drive.tenderdash.nodeKey')).length !== 0;
          const isGenesisPresent = Object.keys(config.get('platform.drive.tenderdash.genesis')).length !== 0;

          const { Volumes: existingVolumes } = await docker.listVolumes();
          const { COMPOSE_PROJECT_NAME: composeProjectName } = config.toEnvs();
          const isDataVolumePresent = existingVolumes.find((v) => v.Name === `${composeProjectName}_drive_tenderdash`);

          if (isValidatorKeyPresent && isNodeKeyPresent
            && isGenesisPresent && isDataVolumePresent) {
            task.skip('Node already initialized');

            return;
          }

          const [validatorKey, nodeKey, genesis, nodeId] = await initializeTenderdashNode(config);

          config.set('platform.drive.tenderdash.nodeId', nodeId);

          if (!isValidatorKeyPresent) {
            config.set('platform.drive.tenderdash.validatorKey', validatorKey);
          }

          if (!isNodeKeyPresent) {
            config.set('platform.drive.tenderdash.nodeKey', nodeKey);
          }

          if (!isGenesisPresent) {
            config.set('platform.drive.tenderdash.genesis', genesis);
          }
        },
      },
    ]);
  }

  return tenderdashInitTask;
}

module.exports = tenderdashInitTaskFactory;
