/* eslint-disable no-console */
import {Listr} from 'listr2';
import {NETWORK_LOCAL} from '../../constants.js';

const MIN_BLOCKS_BEFORE_DKG = 6

/**
 * @param {DockerCompose} dockerCompose
 * @param {createRpcClient} createRpcClient
 * @param {getConnectionHost} getConnectionHost
 * @return {stopNodeTask}
 */
export default function stopNodeTaskFactory(
  dockerCompose,
  createRpcClient,
  getConnectionHost,
) {
  /**
   * Stop node
   * @typedef stopNodeTask
   * @param {Config} config
   *
   * @return {Listr}
   */
  function stopNodeTask(config) {
    return new Listr([
      {
        title: 'Check node is running',
        skip: (ctx) => ctx.isForce,
        task: async (ctx) => {
          const profiles = [];
          if (ctx.platformOnly) {
            profiles.push('platform');
          }

          if (!await dockerCompose.isNodeRunning(config, {profiles})) {
            throw new Error('Node is not running');
          }
        },
      },
      {
        title: 'Check node is participating in DKG',
        enable: (ctx) => !ctx.isForce && config.get('network') !== NETWORK_LOCAL,
        task: async (ctx, task) => {
          const rpcClient = createRpcClient({
            port: config.get('core.rpc.port'),
            user: config.get('core.rpc.user'),
            pass: config.get('core.rpc.password'),
            host: await getConnectionHost(config, 'core', 'core.rpc.host'),
          });

          const {result: dkgInfo} = await rpcClient.quorum('dkginfo');

          const {active_dkgs, next_dkg} = dkgInfo

          if (active_dkgs !== 0 || next_dkg < MIN_BLOCKS_BEFORE_DKG) {
            const agreement = await task.prompt({
              type: 'toggle',
              name: 'confirm',
              header: 'Your node is currently participating in DKG exchange session, restarting may '
                + 'result in PoSE ban.\n Do you want to proceed?',
              message: 'Restart node?',
              enabled: 'Yes',
              disabled: 'No',
            });

            if (!agreement) {
              throw new Error('Node is currently in the DKG window');
            }
          }
        },
      },
      {
        title: 'Save core node time',
        enabled: () => config.get('group') === 'local',
        skip: (ctx) => ctx.isForce,
        task: async () => {
          const rpcClient = createRpcClient({
            port: config.get('core.rpc.port'),
            user: config.get('core.rpc.user'),
            pass: config.get('core.rpc.password'),
            host: await getConnectionHost(config, 'core', 'core.rpc.host'),
          });

          const {result: {mediantime}} = await rpcClient.getBlockchainInfo();

          config.set('core.miner.mediantime', mediantime);
        },
      },
      {
        title: `Stopping ${config.getName()} node`,
        task: async (ctx) => {
          const profiles = [];
          if (ctx.platformOnly) {
            profiles.push('platform');
          }
          await dockerCompose.stop(config, {profiles});
        },
      },
    ]);
  }

  return stopNodeTask;
}
