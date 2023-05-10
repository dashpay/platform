const { Listr } = require('listr2');

/**
 * @param {DockerCompose} dockerCompose
 * @param {createRpcClient} createRpcClient
 * @param {getConnectionHost} getConnectionHost
 * @return {stopNodeTask}
 */
function stopNodeTaskFactory(
  dockerCompose,
  createRpcClient,
  getConnectionHost,
) {
  /**
   * Stop node
   * @typedef stopNodeTask
   * @param {Config} config
   * @param {Object} [options={}]
   * @param {boolean} [options.platformOnly=false]
   *
   * @return {Listr}
   */
  function stopNodeTask(config, options = {}) {
    return new Listr([
      {
        title: 'Check node is running',
        skip: (ctx) => ctx.isForce,
        task: async () => {
          if (!await dockerCompose.isServiceRunning(config.toEnvs(options))) {
            throw new Error('Node is not running');
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
            host: await getConnectionHost(config, 'core'),
          });

          const { result: { mediantime } } = await rpcClient.getBlockchainInfo();

          config.set('core.miner.mediantime', mediantime);
        },
      },
      {
        title: `Stopping ${config.getName()} node`,
        task: async () => dockerCompose.stop(config.toEnvs(options)),
      },
    ]);
  }

  return stopNodeTask;
}

module.exports = stopNodeTaskFactory;
