import { Listr } from 'listr2';

/**
 * @return {configureTenderdashTask}
 */
export default function configureTenderdashTaskFactory() {
  /**
   * @typedef {configureTenderdashTask}
   * @param {Config[]} configGroup
   * @return {Listr}
   */
  function configureTenderdashTask(configGroup) {
    return new Listr([
      {
        task: async () => {
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
                  config.get('platform.drive.abci.validatorSet.quorum.llmqType'),
                );
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
