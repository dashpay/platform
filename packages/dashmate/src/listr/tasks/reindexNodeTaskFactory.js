const {Listr} = require('listr2');
const {Observable} = require('rxjs');
const CoreService = require('../../core/CoreService');

/**
 * @param {Docker} docker
 * @param {DockerCompose} dockerCompose
 * @param {startCore} startCore
 * @param {stopNodeTask} stopNodeTask
 * @param {waitForCoreStart} waitForCoreStart
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
  waitForCoreStart,
  waitForCoreSync,
  createRpcClient,
  renderServiceTemplates,
  writeServiceConfigs,
  configFileRepository,
  configFile,
) {
  /**
   * @typedef {reindexNodeTask}
   * @param {Config} config
   */
  function reindexNodeTask(config) {
    return new Listr([
      {
        title: 'Check services are not running',
        enabled: () => config.get('core.reindex.enable'),
        task: async () => {
          const isRunning = await dockerCompose.isServiceRunning(config.toEnvs());

          if (isRunning) {
            throw new Error('Services is running, stop your nodes first');
          }
        },
      },
      {
        title: 'Set reindex mode',
        enabled: () => config.get('core.reindex.enable'),
        task: async () => {
          config.set('core.reindex.enable', 1);

          // Write configs
          configFileRepository.write(configFile);
          const configFiles = renderServiceTemplates(config);
          writeServiceConfigs(config.getName(), configFiles);
        },
      },
      {
        title: 'Start core',
        task: async (ctx) => {
          let containerId = config.get('core.reindex.containerId', false);

          if (containerId) {
            try {
              await docker.getContainer(containerId).inspect();
            } catch (e) {
              if (e.reason === 'no such container') {
                containerId = null
              }
              throw e
            }
          }

          if (!containerId) {
            ctx.coreService = await startCore(config);
            const containerInfo = await ctx.coreService.dockerContainer.inspect();

            ctx.reindexContainerId = containerInfo.Id;
            config.set('core.reindex.containerId', containerInfo.Id);
            configFileRepository.write(configFile);

            return;
          }

          const container = docker.getContainer(containerId);
          const containerInfo = await container.inspect();
          ctx.reindexContainerId = containerInfo.Id;
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

          const { State } = containerInfo

          if (State.Status === 'paused' || State.Status === 'exited') {
            switch (State.ExitCode) {
              default:
                console.warn(`Reindex container exited with status ${State.ExitCode}, check docker logs of container ${containerId}`);
              // eslint-disable-next-line no-fallthrough
              case 0:
                await container.start();
            }
          }
        },
      },
      {
        title: 'Wait for Core start',
        task: async (ctx) => waitForCoreStart(ctx.coreService),
      },
      {
        task: async (ctx) => new Observable(async (observer) => {
          observer.next(`Reindexing Core for ${config.getName()}`);

          await waitForCoreSync(ctx.coreService, (verificationProgress) => {
            const {percent, blocks, headers} = verificationProgress;

            observer.next(`Reindexing ${config.getName()}... (${(percent * 100).toFixed(4)}%, ${blocks} / ${headers})`);
          });

          await new Promise((res) => setTimeout(res, 2000));

          observer.complete();
        }),
      },
      {
        title: 'Stop services',
        task: async () => {
          const containerId = config.get('core.reindex.containerId', false);
          const container = docker.getContainer(containerId);

          await container.stop();
        },
      },
      {
        title: 'Disable reindex mode',
        task: async () => {
          config.set('core.reindex.enable', 0);
          config.set('core.reindex.containerId', null);

          // Write configs
          configFileRepository.write(configFile);
          const configFiles = renderServiceTemplates(config);
          writeServiceConfigs(config.getName(), configFiles);
        },
      },
    ]);
  }

  return reindexNodeTask;
}

module.exports = reindexNodeTaskFactory;
