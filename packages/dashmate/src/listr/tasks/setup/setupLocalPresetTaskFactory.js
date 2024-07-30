import { Listr } from 'listr2';
import {
  PRESET_LOCAL,
} from '../../../constants.js';
import generateTenderdashNodeKey from '../../../tenderdash/generateTenderdashNodeKey.js';
import deriveTenderdashNodeId from '../../../tenderdash/deriveTenderdashNodeId.js';
import generateRandomString from '../../../util/generateRandomString.js';

/**
 * @param {ConfigFile} configFile
 * @param {configureCoreTask} configureCoreTask
 * @param {configureTenderdashTask} configureTenderdashTask
 * @param {obtainSelfSignedCertificateTask} obtainSelfSignedCertificateTask
 * @param {resolveDockerHostIp} resolveDockerHostIp
 * @param {generateHDPrivateKeys} generateHDPrivateKeys
 * @param {HomeDir} homeDir
 * @param {DockerCompose} dockerCompose
 */
export default function setupLocalPresetTaskFactory(
  configFile,
  configureCoreTask,
  obtainSelfSignedCertificateTask,
  configureTenderdashTask,
  resolveDockerHostIp,
  generateHDPrivateKeys,
  homeDir,
  dockerCompose,
) {
  /**
   * @typedef {setupLocalPresetTask}
   * @return {Listr}
   */
  function setupLocalPresetTask() {
    return new Listr([
      {
        title: 'System requirements',
        task: async () => dockerCompose.throwErrorIfNotInstalled(),
      },
      {
        title: 'Set the number of nodes',
        enabled: (ctx) => ctx.nodeCount === undefined,
        task: async (ctx, task) => {
          ctx.nodeCount = await task.prompt({
            type: 'input',
            message: 'Enter the number of masternodes',
            initial: 3,
            validate: (state) => {
              if (Number.isNaN(+state)) {
                return 'You must set a number of masternodes';
              }

              if (!Number.isInteger(+state)) {
                return 'Must be an integer';
              }

              if (+state < 3) {
                return 'You must set not less than 3';
              }

              return true;
            },
            result: (value) => Number(value),
          });
        },
      },
      {
        title: 'Enable debug logs',
        enabled: (ctx) => ctx.debugLogs === undefined,
        task: async (ctx, task) => {
          ctx.debugLogs = await task.prompt({
            type: 'Toggle',
            message: 'Enable debug logs?',
            enabled: 'yes',
            disabled: 'no',
            initial: 'no',
          });
        },
      },
      {
        title: 'Set the core miner interval',
        enabled: (ctx) => ctx.minerInterval === undefined,
        task: async (ctx, task) => {
          ctx.minerInterval = await task.prompt({
            type: 'input',
            message: 'Enter the interval between core blocks',
            initial: configFile.getConfig('base')
              .get('core.miner.interval'),
            validate: (state) => {
              if (state.match(/\d+(\.\d+)?([ms])/)) {
                return true;
              }

              return 'Please enter a valid integer or decimal duration with m or s units';
            },
          });
        },
      },
      {
        title: 'Create local group configs',
        task: async (ctx) => {
          ctx.configGroup = new Array(ctx.nodeCount)
            .fill(undefined)
            .map((value, i) => `local_${i + 1}`)
            // we need to add one more node (number of masternodes + 1) as a seed node
            .concat(['local_seed'])
            .map((configName) => (
              configFile.isConfigExists(configName)
                ? configFile.getConfig(configName)
                : configFile.createConfig(configName, PRESET_LOCAL)
            ));

          ctx.configGroup.forEach((config) => config.set('group', 'local'));

          configFile.setDefaultGroupName(PRESET_LOCAL);

          const hostDockerInternalIp = await resolveDockerHostIp();

          const subTasks = ctx.configGroup.map((config, i) => (
            {
              title: `Create ${config.getName()} config`,
              task: () => {
                const nodeIndex = i + 1;

                config.set('group', 'local');
                config.set('core.p2p.port', config.get('core.p2p.port') + (i * 100));
                config.set('core.rpc.port', config.get('core.rpc.port') + (i * 100));

                Object.values(config.get('core.rpc.users')).forEach((options) => {
                  // eslint-disable-next-line no-param-reassign
                  options.password = generateRandomString(12);
                });

                config.set('externalIp', hostDockerInternalIp);

                const subnet = config.get('docker.network.subnet')
                  .split('.');
                subnet[2] = nodeIndex;

                config.set('docker.network.subnet', subnet.join('.'));

                // Setup Core debug logs
                const coreLogFilePath = homeDir.joinPath('logs', config.getName(), 'core.log');
                config.set('core.log.file.path', coreLogFilePath);

                if (ctx.debugLogs) {
                  config.set('core.log.file.categories', ['all']);
                }

                // Although not all nodes are miners, all nodes should be aware of
                // the miner interval to be able to sync mocked time
                config.set('core.miner.interval', ctx.minerInterval);

                config.set('dashmate.helper.api.port', config.get('dashmate.helper.api.port') + (i * 100));

                if (config.getName() === 'local_seed') {
                  config.set('description', 'seed node for local network');

                  config.set('core.masternode.enable', false);

                  // Enable miner for the seed node
                  config.set('core.miner.enable', true);

                  // We need them to register masternodes
                  config.set('core.indexes', ['tx', 'address', 'timestamp', 'spent']);

                  // Disable platform for the seed node
                  config.set('platform.enable', false);
                  config.set('platform.drive.tenderdash.mode', 'seed');
                } else {
                  config.set('description', `local node #${nodeIndex}`);

                  config.set('platform.drive.tenderdash.mode', 'validator');

                  const key = generateTenderdashNodeKey();
                  const id = deriveTenderdashNodeId(key);

                  config.set('platform.drive.tenderdash.node.id', id);
                  config.set('platform.drive.tenderdash.node.key', key);

                  config.set('platform.drive.abci.grovedbVisualizer.port', config.get('platform.drive.abci.grovedbVisualizer.port') + (i * 100));
                  config.set('platform.drive.abci.tokioConsole.port', config.get('platform.drive.abci.tokioConsole.port') + (i * 100));
                  config.set('platform.drive.abci.metrics.port', config.get('platform.drive.abci.metrics.port') + (i * 100));
                  config.set('platform.gateway.admin.port', config.get('platform.gateway.admin.port') + (i * 100));
                  config.set('platform.gateway.listeners.dapiAndDrive.port', config.get('platform.gateway.listeners.dapiAndDrive.port') + (i * 100));
                  config.set('platform.gateway.metrics.port', config.get('platform.gateway.metrics.port') + (i * 100));
                  config.set('platform.gateway.rateLimiter.metrics.port', config.get('platform.gateway.rateLimiter.metrics.port') + (i * 100));
                  config.set('platform.drive.tenderdash.p2p.port', config.get('platform.drive.tenderdash.p2p.port') + (i * 100));
                  config.set('platform.drive.tenderdash.rpc.port', config.get('platform.drive.tenderdash.rpc.port') + (i * 100));
                  config.set('platform.drive.tenderdash.pprof.port', config.get('platform.drive.tenderdash.pprof.port') + (i * 100));
                  config.set('platform.drive.tenderdash.metrics.port', config.get('platform.drive.tenderdash.metrics.port') + (i * 100));
                  config.set('platform.drive.tenderdash.moniker', config.name);

                  // Setup logs
                  if (ctx.debugLogs) {
                    const stdoutLogger = config.get('platform.drive.abci.logs.stdout');
                    if (stdoutLogger) {
                      config.set('platform.drive.abci.logs.stdout.level', 'trace');
                      config.set('platform.drive.abci.logs.stdout.format', 'full');
                    }

                    // TODO: Shall we use trace?
                    config.set('platform.drive.tenderdash.log.level', 'debug');
                  }
                }
              },
              options: {
                persistentOutput: true,
              },
            }
          ));

          return new Listr(subTasks);
        },
        options: {
          persistentOutput: true,
        },
      },
      {
        title: 'Configure Core nodes',
        task: (ctx) => configureCoreTask(ctx.configGroup),
      },
      {
        title: 'Configure Tenderdash nodes',
        task: (ctx) => configureTenderdashTask(ctx.configGroup),
      },
      {
        title: 'Configure SSL certificates',
        task: (ctx) => {
          const platformConfigs = ctx.configGroup.filter((config) => config.get('platform.enable'));

          const subTasks = platformConfigs.map((config) => ({
            title: `Generate certificate for ${config.getName()}`,
            task: async () => obtainSelfSignedCertificateTask(config),
          }));

          return new Listr(subTasks);
        },
      },
    ]);
  }

  return setupLocalPresetTask;
}
