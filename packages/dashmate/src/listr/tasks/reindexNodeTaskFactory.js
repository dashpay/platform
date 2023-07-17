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
   * @return {Promise<null|Container>}
   */
  async function getCoreContainer(config) {
    const { docker } = dockerCompose;

    const [containerId] = await dockerCompose
      .getContainersList(generateEnvs(configFile, config), {
        quiet: true,
        all: true,
        filterServiceNames: 'core',
      });

    if (!containerId) {
      return null;
    }

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

          let header;
          if (isNodeRunning) {
            header = `Node found running. The node will be unavailable until reindex is complete.\n`
          } else {
            header = `Node is not running. The node will be automatically started and available after reindex is complete.\n`
          }

          const agreement = await task.prompt({
            type: 'toggle',
            name: 'confirm',
            header,
            message: 'Start reindex?',
            enabled: 'Yes',
            disabled: 'No',
          });

          if (agreement) {
            throw new Error('Opearation is cancelled');
          }
        },
      },
      {
        title: 'Start Core in reindex mode',
        task: async () => {
          // TODO: Set this option through render functions
          // Write dashd.conf with reindex 1
          config.set('core.reindex.enable', 1);

          configFileRepository.write(configFile);
          const configFiles = renderServiceTemplates(config);
          writeServiceConfigs(config.getName(), configFiles);

          const coreContainer = await getCoreContainer(config);
          if (coreContainer) {
            const info = await coreContainer.inspect();

            // If core is running, we need to stop it first
            if (info.State.Status !== 'exited') {
              await coreContainer.restart();

              return;
            }
          }

          return startNodeTask(config);
        },
      },
      {
        title: 'Wait for Core start',
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

          // TODO: Do not use config
          config.set('core.reindex.enable', 0);

          // Write configs
          configFileRepository.write(configFile);
          const configFiles = renderServiceTemplates(config);
          writeServiceConfigs(config.getName(), configFiles);
        },
      },
      {
        title: 'Wait for Core reindex finished',
        enabled: (ctx) => !ctx.isDetached,
        task: async (ctx) => new Observable(async (observer) => {
          observer.next(`Starting reindex...`);

          await waitForCoreSync(ctx.coreService, (verificationProgress) => {
            const { percent, blocks, headers } = verificationProgress;

            observer.next(`${(percent * 100).toFixed(4)}%, ${blocks} / ${headers}`);
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
