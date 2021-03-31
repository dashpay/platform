const { Listr } = require('listr2');

/**
 * @param {tenderdashInitTask} tenderdashInitTask
 * @param {renderServiceTemplates} renderServiceTemplates
 * @param {writeServiceConfigs} writeServiceConfigs
 * @param {resolveDockerHostIp} resolveDockerHostIp
 * @return {configureTenderdashTask}
 */
function configureTenderdashTaskFactory(
  tenderdashInitTask,
  renderServiceTemplates,
  writeServiceConfigs,
  resolveDockerHostIp,
) {
  /**
   * @typedef {configureTenderdashTask}
   * @param {Config[]} configGroup
   * @return {Listr}
   */
  function configureTenderdashTask(configGroup) {
    return new Listr([
      {
        task: async (ctx) => {
          if (!ctx.hostDockerInternalIp) {
            ctx.hostDockerInternalIp = await resolveDockerHostIp();
          }

          const masternodeConfigs = configGroup.filter((config) => config.get('core.masternode.enable'));

          const subTasks = masternodeConfigs.map((config) => ({
            title: `Initialize ${config.getName()} Tenderdash`,
            task: () => tenderdashInitTask(config),
          }));

          // Interconnect Tenderdash nodes
          subTasks.push({
            task: async () => {
              const validators = masternodeConfigs.map((config) => {
                const validatorKey = config.get('platform.drive.tenderdash.validatorKey');

                return {
                  address: validatorKey.address,
                  pub_key: validatorKey.pub_key,
                  power: '1',
                  name: config.getName(),
                };
              });

              const randomChainIdPart = Math.floor(Math.random() * 60) + 1;
              const chainId = `dash_masternode_local_${randomChainIdPart}`;

              const genesisTime = masternodeConfigs[0].get('platform.drive.tenderdash.genesis.genesis_time');

              masternodeConfigs.forEach((config, index) => {
                config.set('platform.drive.tenderdash.genesis.genesis_time', genesisTime);
                config.set('platform.drive.tenderdash.genesis.chain_id', chainId);
                config.set(
                  'platform.drive.tenderdash.genesis.initial_core_chain_locked_height',
                  ctx.initialCoreChainLockedHeight,
                );

                const p2pPeers = masternodeConfigs
                  .filter((_, i) => i !== index)
                  .map((innerConfig) => {
                    const nodeId = innerConfig.get('platform.drive.tenderdash.nodeId');
                    const port = innerConfig.get('platform.drive.tenderdash.p2p.port');

                    return {
                      id: nodeId,
                      host: ctx.hostDockerInternalIp,
                      port,
                    };
                  });

                config.set('platform.drive.tenderdash.p2p.persistentPeers', p2pPeers);
                config.set('platform.drive.tenderdash.genesis.validators', validators);

                const configFiles = renderServiceTemplates(config);
                writeServiceConfigs(config.getName(), configFiles);
              });
            },
          });

          return new Listr(subTasks);
        },
      },
    ]);
  }

  return configureTenderdashTask;
}

module.exports = configureTenderdashTaskFactory;
