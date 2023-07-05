const { Listr } = require('listr2');
const { Observable } = require('rxjs');
const DockerStatusEnum = require('../../status/enums/dockerStatus');
const generateEnvs = require('../../util/generateEnvs');
const CoreService = require('../../core/CoreService');

/**
 * @param {DockerCompose} dockerCompose
 * @param {startCore} startCore
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
  startCore,
  stopNodeTask,
  waitForCoreStart,
  waitForCoreSync,
  createRpcClient,
  configFileRepository,
  configFile,
  getConnectionHost,
  storage,
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
        title: 'Check reindex is running',
        task: async (ctx) => {
          const { docker } = dockerCompose;

          const containerId = await storage.getItem('containerId');

          if (containerId) {
            try {
              const info = await docker.getContainer(containerId).inspect();
              const { State } = info;

              switch (State) {
                case DockerStatusEnum.running:
                  ctx.containerId = containerId;
                  ctx.coreService = new CoreService(
                    config,
                    createRpcClient(
                      {
                        port: config.get('core.rpc.port'),
                        user: config.get('core.rpc.user'),
                        pass: config.get('core.rpc.password'),
                        host: await getConnectionHost(config, 'core'),
                      },
                    ),
                    docker.getContainer(containerId),
                  );
                  break;
                case DockerStatusEnum.exited:
                  // todo check exit code
                  // remove from db and exit
                  await storage.setItem('containerId', null);
                  break;
                default:
                  throw new Error('Unexpected reindex container status');
              }
            } catch (e) {
              if (e.reason !== 'no such container') {
                throw e;
              }
            }
          }
        },
      },
      {
        title: 'Start core',
        enabled: (ctx) => !ctx.containerId,
        task: async (ctx) => {
          ctx.coreService = await startCore(config, { reindex: true });
          ctx.containerId = ctx.coreService.dockerContainer.id;
        },
      },
      {
        title: 'Wait for Core start',
        enabled: (ctx) => !ctx.detach,
        task: async (ctx) => waitForCoreStart(ctx.coreService),
      },
      {
        enabled: (ctx) => !ctx.detach,
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
      {
        title: 'Stop services',
        enabled: (ctx) => !ctx.detach,
        task: async (ctx) => {
          const container = dockerCompose.docker.getContainer(ctx.containerId);

          await container.stop();
        },
      },
    ]);
  }

  return reindexNodeTask;
}

module.exports = reindexNodeTaskFactory;
