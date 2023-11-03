const { Listr } = require('listr2');
const { getLatestProtocolVersion } = require('@dashevo/wasm-dpp');

/**
 * @param {renderServiceTemplates} renderServiceTemplates
 * @param {writeServiceConfigs} writeServiceConfigs
 * @return {configureTenderdashTask}
 */
function configureTenderdashTaskFactory(
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
          const platformConfigs = configGroup.filter((config) => config.get('platform.enable'));

          const subTasks = [];

          // Interconnect Tenderdash nodes
          subTasks.push({
            task: async () => {
              const randomChainIdPart = Math.floor(Math.random() * 60) + 1;
              const chainId = `dashmate_local_${randomChainIdPart}`;

              const genesisTime = new Date().toISOString();

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
                    const nodeId = innerConfig.get('platform.drive.tenderdash.node.id');
                    const port = innerConfig.get('platform.drive.tenderdash.p2p.port');

                    return {
                      id: nodeId,
                      host: config.get('externalIp'),
                      port,
                    };
                  });

                config.set('platform.drive.tenderdash.p2p.persistentPeers', p2pPeers);

                config.set(
                  'platform.drive.tenderdash.genesis.validator_quorum_type',
                  config.get('platform.drive.abci.validatorSet.llmqType'),
                );

                config.set(
                  'platform.drive.tenderdash.genesis.consensus_params.version.app_version',
                  getLatestProtocolVersion().toString(),
                );

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
