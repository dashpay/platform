const { Listr } = require('listr2');
const generateEnvs = require('../../util/generateEnvs');

/**
 * @param {DockerCompose} dockerCompose
 * @param {createRpcClient} createRpcClient
 * @param {getConnectionHost} getConnectionHost
 * @param {ConfigFile} configFile
 * @return {stopNodeTask}
 */
function stopNodeTaskFactory(
  dockerCompose,
  createRpcClient,
  getConnectionHost,
  configFile,
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
          if (!await dockerCompose.isNodeRunning(
            generateEnvs(configFile, config, { platformOnly: ctx.platformOnly }),
          )) {
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
        task: async (ctx) => dockerCompose.stop(
          generateEnvs(configFile, config, { platformOnly: ctx.platformOnly }),
        ),
      },
    ]);
  }

  return stopNodeTask;
}

module.exports = stopNodeTaskFactory;
