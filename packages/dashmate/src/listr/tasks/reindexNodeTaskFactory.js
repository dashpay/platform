const {Listr} = require('listr2');
const {Observable} = require('rxjs')
const CoreService = require('../../core/CoreService')

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
        title: 'Set core to reindex on next run',
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
            ctx.coreService = await startCore(config)
            const containerInfo = await ctx.coreService.dockerContainer.inspect()

            ctx.reindexContainerId = containerInfo.Id
            config.set('core.reindexContainerId', containerInfo.Id)
            configFileRepository.write(configFile)

            return
          }

          const container = docker.getContainer(containerId);
          const containerInfo = await container.inspect()
          ctx.reindexContainerId = containerInfo.Id
          ctx.coreService = new CoreService(
            config,
            createRpcClient(
              {
                port: config.get('core.rpc.port'),
                user: config.get('core.rpc.user'),
                pass: config.get('core.rpc.password'),
              },
            ),
            dockerCompose.docker.getContainer(containerId),
          );

          const {State} = await container.inspect()

          if (State.Status === "paused" || State.Status === "exited") {
            switch (State.ExitCode) {
              default:
                console.warn(`Reindex container exited with status ${State.ExitCode}, check docker logs of container ${containerId}`)
              case 0:
                await container.start();
            }
          }
        }
      },
      {
        task: async (ctx) => {
          return new Observable(async observer => {
            observer.next('Reindexing dashcore ' + config.getName())

            await waitForCoreSync(ctx.coreService, (verificationProgress) => {
              const {percent, blocks, headers} = verificationProgress

              observer.next(`Reindexing ${config.getName()}... (${(percent * 100).toFixed(4)}%, ${blocks} / ${headers})`)
            })

            await new Promise((res)=> setTimeout(res, 2000))

            observer.complete()
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
        title: 'Set core to disable reindex on next run',
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
