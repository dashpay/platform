const { Listr } = require('listr2');

/**
 * @param {DockerCompose} dockerCompose
 * @param {createRpcClient} createRpcClient
 * @return {stopNodeTask}
 */
function stopNodeTaskFactory(
  dockerCompose,
  createRpcClient,
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
        task: async () => {
          if (!await dockerCompose.isServiceRunning(config.toEnvs())) {
            console.log(`Node is not running`);
            this.exit()
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
          });

          const { result: { mediantime } } = await rpcClient.getBlockchainInfo();

          config.set('core.miner.mediantime', mediantime);
        },
      },
      {
        title: `Stopping ${config.getName()} node`,
        task: async () => dockerCompose.stop(config.toEnvs()),
      },
    ]);
  }

  return stopNodeTask;
}

module.exports = stopNodeTaskFactory;
