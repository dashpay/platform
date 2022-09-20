const {Listr} = require('listr2');

/**
 * @param {Docker} docker
 * @param {DockerCompose} dockerCompose
 * @param {startCore} startCore
 * @param {stopNodeTask} stopNodeTask
 * @param {waitForCoreSync} waitForCoreSync
 * @param {createRpcClient} createRpcClient
 * @param {renderServiceTemplates} renderServiceTemplates
 * @param {writeServiceConfigs} writeServiceConfigs
 * @param {configFileRepository} configFileRepository
 * @param {configFile} configFile
 * @return {reindexNodeTask}
 */
function reindexNodeTaskFactory(
  docker,
  dockerCompose,
  startCore,
  stopNodeTask,
  waitForCoreSync,
  createRpcClient,
  renderServiceTemplates,
  writeServiceConfigs,
  configFileRepository,
  configFile
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
        task: async () => {
          const isRunning = await dockerCompose.isServiceRunning(config.toEnvs())

          if (isRunning) {
            throw new Error('Services is running, stop your nodes first')
          }
        }
      },
      {
        title: 'Set core reindex env to 1',
        enabled: () => config.get('core.reindex') === 0,
        task: async () => {
          config.set('core.reindex', 1)

          // Write configs
          configFileRepository.write(configFile)
          const configFiles = renderServiceTemplates(config);
          writeServiceConfigs(config.getName(), configFiles);
        }
      },
      {
        title: 'Start core',
        task: async (ctx) => {
          const containerId = config.get('core.reindexContainerId', false)

          if (!containerId) {
            const coreService = await startCore(config)
            const containerInfo = await coreService.dockerContainer.inspect()

            ctx.reindexContainerId = containerInfo.Id
            config.set('core.reindexContainerId', containerInfo.Id)
            configFileRepository.write(configFile)

            return
          }

          const container = docker.getContainer(containerId);
          const {State} = await container.inspect()

          if (State.Status === "paused" || State.Status === "exited") {
            switch (State.ExitCode) {
              // 127 means out of memory or something, so we would want to spin in it up again
              case 127:
              case 0:
                await container.start();
                break;
              default:
                throw new Error(`Reindex container exited with status ${State.ExitCode}, look docker logs of container ${containerId}`)
            }
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
            task.title = `Reindexing ${ctx.reindexContainerId}... (${(percent * 100).toFixed(4)}%, ${blocks} / ${headers})`
          })
        },
      },
      {
        title: 'Stop services',
        task: async () => {
          const containerId = config.get('core.reindexContainerId', false)
          const container = docker.getContainer(containerId);

          await container.stop()
        },
      },
      {
        title: 'Set reindex back to zero',
        task: async () => {
          config.set('core.reindex', 0)
          config.set('core.reindexContainerId', null)

          // Write configs
          configFileRepository.write(configFile)
          const configFiles = renderServiceTemplates(config);
          writeServiceConfigs(config.getName(), configFiles);
        }
      },
    ]);
  }

  return reindexNodeTask;
}

module.exports = reindexNodeTaskFactory;
