const { Listr } = require('listr2');
const { Observable } = require('rxjs');
const path = require('path');
const generateEnvs = require('../../util/generateEnvs');
const CoreService = require('../../core/CoreService');
const { TEMPLATES_DIR } = require('../../constants');

/**
 * @param {DockerCompose} dockerCompose
 * @param {startNodeTask} startNodeTask
 * @param {restartNodeTask} restartNodeTask
 * @param {waitForCoreStart} waitForCoreStart
 * @param {waitForCoreSync} waitForCoreSync
 * @param {createRpcClient} createRpcClient
 * @param {renderTemplate} renderTemplate
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
  restartNodeTask,
  waitForCoreStart,
  waitForCoreSync,
  createRpcClient,
  renderTemplate,
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
        enabled: (ctx) => !ctx.isForce,
        task: async (ctx, task) => {
          const isNodeRunning = await dockerCompose.isNodeRunning(generateEnvs(configFile, config));

          let header;
          if (isNodeRunning) {
            header = 'Node found running. The node will be unavailable until reindex is complete.\n';
          } else {
            header = 'Node is not running. The node will be automatically started and available after reindex is complete.\n';
          }

          const agreement = await task.prompt({
            type: 'toggle',
            name: 'confirm',
            header,
            message: 'Start reindex?',
            enabled: 'Yes',
            disabled: 'No',
          });

          if (!agreement) {
            throw new Error('Operation is cancelled');
          }
        },
      },
      {
        title: 'Start Core in reindex mode',
        task: async () => {
          const configPath = 'core/dash.conf';
          const templatePath = `${configPath}.dot`;

          const serviceConfig = renderTemplate(path.join(TEMPLATES_DIR, templatePath),
            { ...config.options, reindex: true });
          writeServiceConfigs(config.getName(), { [configPath]: serviceConfig });

          const coreContainer = await getCoreContainer(config);

          // if core container found
          if (coreContainer) {
            const info = await coreContainer.inspect();

            if (info.State.Status !== 'exited') {
              await coreContainer.restart();
              return Promise.resolve();
            }
            await coreContainer.start();
            return Promise.resolve();
          }
          const isNodeRunning = await dockerCompose.isNodeRunning(generateEnvs(configFile, config));

          // if container not found, but node is running (core container was removed manually by user)
          if (isNodeRunning) {
            return restartNodeTask(config);
          }
          // start node in case nothing is running
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

          // Write configs
          const configFiles = renderServiceTemplates(config);
          writeServiceConfigs(config.getName(), configFiles);
        },
      },
      {
        title: 'Wait for Core reindex finished',
        enabled: (ctx) => !ctx.isDetached,
        task: async (ctx) => new Observable(async (observer) => {
          observer.next('Starting reindex...');

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
