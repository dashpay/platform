const { Listr } = require('listr2');
const getSeedNodeConfig = require('../../../../util/getSeedNodeConfig');

/**
 *
 * @param {activateCoreSpork} activateCoreSpork
 * @param {waitForCoreQuorum} waitForCoreQuorum
 * @param {createRpcClient} createRpcClient
 * @return {activateSporksTask}
 */
function activateSporksTaskFactory(
  activateCoreSpork,
  waitForCoreQuorum,
  createRpcClient,
) {
  /**
   * @typedef activateSporksTask
   * @param {Config[]} configGroup
   * @return {Listr}
   */
  function activateSporksTask(configGroup) {
    return new Listr([
      {
        title: 'Enable sporks',
        task: async (ctx) => {
          const seedConfig = getSeedNodeConfig(configGroup);

          ctx.rpcClient = createRpcClient({
            port: seedConfig.get('core.rpc.port'),
            user: seedConfig.get('core.rpc.user'),
            pass: seedConfig.get('core.rpc.password'),
          });

          const sporks = [
            'SPORK_2_INSTANTSEND_ENABLED',
            'SPORK_3_INSTANTSEND_BLOCK_FILTERING',
            'SPORK_9_SUPERBLOCKS_ENABLED',
            'SPORK_17_QUORUM_DKG_ENABLED',
            'SPORK_19_CHAINLOCKS_ENABLED',
          ];

          await Promise.all(
            sporks.map((spork) => (
              activateCoreSpork(ctx.rpcClient, spork))),
          );
        },
      },
      {
        title: 'Waiting for quorums',
        task: async (ctx) => {
          const seedConfig = getSeedNodeConfig(configGroup);

          await waitForCoreQuorum(ctx.rpcClient, seedConfig.get('network'));
        },
      },
    ]);
  }

  return activateSporksTask;
}

module.exports = activateSporksTaskFactory;
