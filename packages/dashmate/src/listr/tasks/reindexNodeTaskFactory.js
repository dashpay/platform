const {Listr} = require('listr2');

/**
 * @param {DockerCompose} dockerCompose
 * @param {startNodeTask} startNodeTask
 * @param {stopNodeTask} stopNodeTask
 * @param {waitForCoreSync} waitForCoreSync
 * @param {createRpcClient} createRpcClient
 * @param {renderServiceTemplates} renderServiceTemplates
 * @param {writeServiceConfigs} writeServiceConfigs
 * @return {reindexNodeTask}
 */
function reindexNodeTaskFactory(
  dockerCompose,
  startNodeTask,
  stopNodeTask,
  waitForCoreSync,
  createRpcClient,
  renderServiceTemplates,
  writeServiceConfigs
) {
  /**
   * @typedef {reindexNodeTask}
   * @param {Config} config
   */
  function reindexNodeTask(config) {
    return new Listr([
      {
        title: 'Check services are not running',
        enabled: () => config.get('core.reindex') === 0,
        task: async (ctx, task) => {
          const isRunning = await dockerCompose.isServiceRunning(config.toEnvs())

          if (isRunning) {
            task.title = 'Stopping services'
            return stopNodeTask(config)
          }
        }
      },
      {
        title: 'Set core reindex env to 1',
        enabled: () => config.get('core.reindex') === 0,
        task: async () => {
          config.set('core.reindex', 1)

          // Write configs
          const configFiles = renderServiceTemplates(config);
          writeServiceConfigs(config.getName(), configFiles);
        }
      },
      {
        title: 'Start services',
        enabled: () => config.get('core.reindex') === 0,
        task: async () => {
          const isRunning = await dockerCompose.isServiceRunning(config.toEnvs())

          if (!isRunning) {
            return startNodeTask(config)
          }
        }
      },
      {
        title: `Wait for the services to be ready`,
        task: async (ctx, task) => {
          const rpcClient = createRpcClient(
            {
              port: config.get('core.rpc.port'),
              user: config.get('core.rpc.user'),
              pass: config.get('core.rpc.password'),
            })

          return waitForCoreSync(rpcClient, (verificationProgress) => {
            const {percent, blocks, headers} = verificationProgress
            task.title = `Reindexing... (${(percent * 100).toFixed(4)}%, ${blocks} / ${headers})`
          })
        },
      },
      {
        title: 'Stop services',
        task: async () => {
          await stopNodeTask(config)
        },
      },
      {
        title: 'Set reindex back to zero',
        task: () => {
          config.set('core.reindex', 0)

          // Write configs
          const configFiles = renderServiceTemplates(config);
          writeServiceConfigs(config.getName(), configFiles);
        }
      },
    ]);
  }

  return reindexNodeTask;
}

module.exports = reindexNodeTaskFactory;
