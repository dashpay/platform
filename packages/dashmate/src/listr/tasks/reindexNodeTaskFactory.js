import { Listr } from 'listr2';
import { Observable } from 'rxjs';
import path from 'path';
import { TEMPLATES_DIR } from '../../constants.js';
import CoreService from '../../core/CoreService.js';

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
 * @param {getConnectionHost} getConnectionHost
 * @param {Docker} docker
 * @return {reindexNodeTask}
 */
export default function reindexNodeTaskFactory(
  dockerCompose,
  startNodeTask,
  restartNodeTask,
  waitForCoreStart,
  waitForCoreSync,
  createRpcClient,
  renderTemplate,
  renderServiceTemplates,
  writeServiceConfigs,
  getConnectionHost,
  docker,
) {
  /**
   * Gets dashcore docker container from the node
   * @param config
   * @return {Promise<null|Container>}
   */
  async function getCoreContainer(config) {
    const [containerId] = await dockerCompose
      .getContainerIds(config, {
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
          const isNodeRunning = await dockerCompose.isNodeRunning(config);

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
          // Temporary (for the first run) set reindex=1 to dashd.conf
          const configPath = 'core/dash.conf';
          const templatePath = `${configPath}.dot`;

          const serviceConfig = renderTemplate(
            path.join(TEMPLATES_DIR, templatePath),
            { ...config.options, reindex: true },
          );

          writeServiceConfigs(config.getName(), { [configPath]: serviceConfig });

          // Restart or start node (including Core container) to apply
          // reindex=1 from dashd.conf
          const isNodeRunning = await dockerCompose.isNodeRunning(config);

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
          // Wait until Core is started
          const rpcClient = createRpcClient({
            port: config.get('core.rpc.port'),
            user: 'dashmate',
            pass: config.get('core.rpc.users.dashmate.password'),
            host: await getConnectionHost(config, 'core', 'core.rpc.host'),
          });

          const container = await getCoreContainer(config);

          if (!container) {
            throw new Error('Core container not found');
          }

          ctx.coreService = new CoreService(config, rpcClient, container);

          await waitForCoreStart(ctx.coreService);

          // When Core is started remove reindex=1 from dashd.conf
          // rendering service templates without additional variables
          const configFiles = renderServiceTemplates(config);
          writeServiceConfigs(config.getName(), configFiles);
        },
      },
      {
        title: 'Reindex Core',
        enabled: (ctx) => !ctx.isDetached,
        task: async (ctx) => new Observable(async (observer) => {
          // Show reindex progeress if not detached
          observer.next('Starting reindex...');

          await waitForCoreSync(ctx.coreService, (verificationProgress) => {
            const { percent, blocks, headers } = verificationProgress;

            observer.next(`${(percent * 100).toFixed(4)}%, ${blocks} / ${headers}`);
          });

          await new Promise((res) => { setTimeout(res, 2000); });

          observer.complete();
        }),
      },
    ]);
  }

  return reindexNodeTask;
}
