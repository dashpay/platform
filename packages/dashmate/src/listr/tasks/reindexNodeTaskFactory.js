const { Listr } = require('listr2');
const { Observable } = require('rxjs');
const generateEnvs = require('../../util/generateEnvs');
const CoreService = require('../../core/CoreService');

/**
 * @param {DockerCompose} dockerCompose
 * @param {startNodeTask} startNodeTask
 * @param {stopNodeTask} stopNodeTask
 * @param {waitForCoreStart} waitForCoreStart
 * @param {waitForCoreSync} waitForCoreSync
 * @param {createRpcClient} createRpcClient
 * @param {renderServiceTemplates} renderServiceTemplates
 * @param {writeServiceConfigs} writeServiceConfigs
 * @param {configFileRepository} configFileRepository
 * @param {ConfigFile} configFile
 * @param {getConnectionHost} getConnectionHost
 * @return {reindexNodeTask}
 */
function reindexNodeTaskFactory(
  dockerCompose,
  startNodeTask,
  stopNodeTask,
  waitForCoreStart,
  waitForCoreSync,
  createRpcClient,
  renderServiceTemplates,
  writeServiceConfigs,
  configFileRepository,
  configFile,
  getConnectionHost,
) {
  /**
   * Gets dashcore docker container from the node
   * @param config
   * @return {Promise<*>}
   */
  async function getCoreContainer(config) {
    const { docker } = dockerCompose;

    const [containerId] = await dockerCompose
      .getContainersList(generateEnvs(configFile, config), {
        quiet: true,
        all: true,
        filterServiceNames: 'core',
      });

    return docker.getContainer(containerId);
  }

  /**
   * @typedef {reindexNodeTask}
   * @param {Config} config
   */
  function reindexNodeTask(config) {
    return new Listr([
      {
        title: 'Check services are not running',
        task: async (ctx, task) => {
          const isNodeRunning = await dockerCompose.isNodeRunning(generateEnvs(configFile, config));

          if (isNodeRunning) {
            ctx.coreContainer = await getCoreContainer(config);

            const info = await ctx.coreContainer.inspect();

            // If core is running, we need to stop it first
            if (info.State.Status !== 'exited') {
              let agreed;

              if (!ctx.isDetached) {
                const agreement = await task.prompt({
                  type: 'toggle',
                  name: 'confirm',
                  header: `Node found running, Dash Core will be restarted and the node will be unavailable until reindex is complete:

Select "No" to cancel operation.\n`,
                  message: 'Stop Dash Core and proceed to reindex?',
                  enabled: 'Yes',
                  disabled: 'No',
                });

                agreed = agreement === 'true';
              }

              if (ctx.isDetached || agreed) {
                await ctx.coreContainer.stop();
              } else {
                // eslint-disable-next-line no-param-reassign
                task.title = 'Cancelled';
                ctx.cancel = true;
              }
            }
          }
        },
      },
      {
        enabled: (ctx) => !ctx.cancel,
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
        enabled: (ctx) => !ctx.cancel,
        task: async (ctx) => {
          if (ctx.coreContainer) {
            return ctx.coreContainer.start();
          }

          return startNodeTask(config);
        },
      },
      {
        title: 'Wait for Core start',
        enabled: (ctx) => !ctx.cancel,
        task: async (ctx) => {
          const rpcClient = createRpcClient({
            port: config.get('core.rpc.port'),
            user: config.get('core.rpc.user'),
            pass: config.get('core.rpc.password'),
            host: await getConnectionHost(config, 'core'),
          });

          const container = await getCoreContainer(config);

          ctx.coreService = new CoreService(config, rpcClient, container);

          await waitForCoreStart(ctx.coreService);
        },
      },
      {
        enabled: (ctx) => !ctx.cancel,
        task: async () => {
          config.set('core.reindex.enable', 0);

          // Write configs
          configFileRepository.write(configFile);
          const configFiles = renderServiceTemplates(config);
          writeServiceConfigs(config.getName(), configFiles);
        },
      },
      {
        enabled: (ctx) => !ctx.cancel && !ctx.isDetached,
        task: async (ctx) => new Observable(async (observer) => {
          observer.next(`Reindexing Core for ${config.getName()}`);

          await waitForCoreSync(ctx.coreService, (verificationProgress) => {
            const { percent, blocks, headers } = verificationProgress;

            observer.next(`Reindexing ${config.getName()}... (${(percent * 100).toFixed(4)}%, ${blocks} / ${headers})`);
          });

          await new Promise((res) => setTimeout(res, 2000));

          observer.complete();
        }),
      },
    ]);
  }

  return reindexNodeTask;
}

module.exports = reindexNodeTaskFactory;
