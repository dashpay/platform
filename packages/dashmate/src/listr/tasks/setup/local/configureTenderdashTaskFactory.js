const { Listr } = require('listr2');

/**
 * @param {tenderdashInitTask} tenderdashInitTask
 * @param {renderServiceTemplates} renderServiceTemplates
 * @param {writeServiceConfigs} writeServiceConfigs
 * @return {configureTenderdashTask}
 */
function configureTenderdashTaskFactory(
  tenderdashInitTask,
  renderServiceTemplates,
  writeServiceConfigs,
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
          const platformConfigs = configGroup.filter((config) => config.has('platform'));

          const subTasks = platformConfigs.map((config) => ({
            title: `Initialize ${config.getName()} Tenderdash`,
            task: () => tenderdashInitTask(config),
          }));

          // Interconnect Tenderdash nodes
          subTasks.push({
            task: async () => {
              const randomChainIdPart = Math.floor(Math.random() * 60) + 1;
              const chainId = `dash_masternode_local_${randomChainIdPart}`;

              const genesisTime = platformConfigs[0].get('platform.drive.tenderdash.genesis.genesis_time');

              platformConfigs.forEach((config, index) => {
                config.set('platform.drive.tenderdash.genesis.genesis_time', genesisTime);
                config.set('platform.drive.tenderdash.genesis.chain_id', chainId);
                config.set(
                  'platform.drive.tenderdash.genesis.initial_core_chain_locked_height',
                  ctx.initialCoreChainLockedHeight,
                );

                const p2pPeers = platformConfigs
                  .filter((_, i) => i !== index)
                  .map((innerConfig) => {
                    const nodeId = innerConfig.get('platform.drive.tenderdash.nodeId');
                    const port = innerConfig.get('platform.drive.tenderdash.p2p.port');

                    return {
                      id: nodeId,
                      host: config.get('externalIp'),
                      port,
                    };
                  });

                config.set('platform.drive.tenderdash.p2p.persistentPeers', p2pPeers);

                config.set(
                  'platform.drive.tenderdash.genesis.quorum_type',
                  config.get('platform.drive.abci.validatorSet.llmqType').toString(),
                );

                config.set('platform.drive.tenderdash.genesis.quorum_hash', null);

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
