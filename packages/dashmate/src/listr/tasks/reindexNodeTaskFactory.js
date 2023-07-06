const {Listr} = require('listr2');
const {Observable} = require('rxjs');
const DockerStatusEnum = require('../../status/enums/dockerStatus');
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
   * @typedef {reindexNodeTask}
   * @param {Config} config
   */
  function reindexNodeTask(config) {
    return new Listr([
      {
        title: 'Check services are not running',
        task: async () => {
          const isRunning = await dockerCompose.isNodeRunning(generateEnvs(configFile, config));

          if (isRunning) {
            throw new Error('Services is running, stop your nodes first');
          }
        },
      },
      {
        enabled: () => true,
        task: async () => {
          config.set('core.reindex.enable', 1);

          // Write configs
          configFileRepository.write(configFile);
          const configFiles = renderServiceTemplates(config);
          writeServiceConfigs(config.getName(), configFiles);
        },
      },
      {
        title: 'Start node',
        enabled: (ctx) => true,
        task: async (ctx) => {
          return startNodeTask(config)
        },
      },
      {
        title: 'Wait for Core start',
        enabled: (ctx) => true,
        task: async (ctx) => {
          const {docker} = dockerCompose;

          const rpcClient = createRpcClient({
            port: config.get('core.rpc.port'),
            user: config.get('core.rpc.user'),
            pass: config.get('core.rpc.password'),
            host: await getConnectionHost(config, 'core'),
          });

          const [containerId] = await dockerCompose
            .getContainersList(generateEnvs(configFile, config), {
              quiet: true,
              filterServiceNames: 'core'
            })

          const container = docker.getContainer(containerId)
          ctx.coreService = new CoreService(config, rpcClient, container)

          await waitForCoreStart(ctx.coreService)
        },
      },
      {
        enabled: () => true,
        task: async () => {
          config.set('core.reindex.enable', 0);

          // Write configs
          configFileRepository.write(configFile);
          const configFiles = renderServiceTemplates(config);
          writeServiceConfigs(config.getName(), configFiles);
        },
      },
      {
        enabled: (ctx) => !ctx.detach,
        task: async (ctx) => new Observable(async (observer) => {
          observer.next(`Reindexing Core for ${config.getName()}`);

          await waitForCoreSync(ctx.coreService, (verificationProgress) => {
            const {percent, blocks, headers} = verificationProgress;

            observer.next(`Reindexing ${config.getName()}... (${(percent * 100).toFixed(4)}%, ${blocks} / ${headers})`);
          });

          await new Promise((res) => setTimeout(res, 2000));

          observer.complete();
        }),
      }
    ]);
  }

  return reindexNodeTask;
}

module.exports = reindexNodeTaskFactory;
