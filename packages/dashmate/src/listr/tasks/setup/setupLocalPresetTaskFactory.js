const { Listr } = require('listr2');
const { PRESET_LOCAL } = require('../../../constants');

/**
 * @param {ConfigFile} configFile
 * @param {configureCoreTask} configureCoreTask
 * @param {configureTenderdashTask} configureTenderdashTask
 * @param {configureTenderdashTask} initializePlatformTask
 * @param {resolveDockerHostIp} resolveDockerHostIp
 */
function setupLocalPresetTaskFactory(
  configFile,
  configureCoreTask,
  configureTenderdashTask,
  initializePlatformTask,
  resolveDockerHostIp,
) {
  /**
   * @typedef {setupLocalPresetTask}
   * @return {Listr}
   */
  function setupLocalPresetTask() {
    return new Listr([
      {
        title: 'Set the number of nodes',
        enabled: (ctx) => ctx.nodeCount === null,
        task: async (ctx, task) => {
          ctx.nodeCount = await task.prompt({
            type: 'Numeral',
            message: 'Enter the number of masternodes',
            initial: 3,
            float: false,
            min: 3,
            validate: (state) => {
              if (+state < 3) {
                return 'You must set not less than 3';
              }

              return true;
            },
          });
        },
      },
      {
        title: 'Enable debug logs',
        enabled: (ctx) => ctx.debugLogs === null,
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
        enabled: (ctx) => ctx.minerInterval === null,
        task: async (ctx, task) => {
          ctx.minerInterval = await task.prompt({
            type: 'input',
            message: 'Enter the interval between core blocks',
            initial: configFile.getConfig('base').options.core.miner.interval,
            validate: (state) => {
              if (state.match(/\d+(\.\d+)?(m|s)/)) {
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

          const hostDockerInternalIp = await resolveDockerHostIp();

          const subTasks = ctx.configGroup.map((config, i) => (
            {
              title: `Create ${config.getName()} config`,
              task: () => {
                const nodeIndex = i + 1;

                config.set('group', 'local');
                config.set('core.p2p.port', 20001 + (i * 100));
                config.set('core.rpc.port', 20002 + (i * 100));
                config.set('externalIp', hostDockerInternalIp);

                if (ctx.debugLogs) {
                  config.set('core.debug', 1);
                  config.set('platform.drive.abci.log.stdout.level', 'trace');
                  config.set('platform.drive.tenderdash.log.level', {
                    '*': 'debug',
                  });
                }

                // Although not all nodes are miners, all nodes should be aware of
                // the miner interval to be able to sync mocked time
                config.set('core.miner.interval', ctx.minerInterval);

                if (config.getName() === 'local_seed') {
                  config.set('description', 'seed node for local network');

                  config.set('core.masternode.enable', false);
                  config.set('core.miner.enable', true);

                  // Enable miner for the seed node
                  config.set('core.miner.enable', true);

                  // Disable platform for the seed node
                  config.set('platform', undefined);
                } else {
                  config.set('description', `local node #${nodeIndex}`);

                  config.set('platform.dapi.envoy.http.port', 3000 + (i * 100));
                  config.set('platform.dapi.envoy.grpc.port', 3010 + (i * 100));
                  config.set('platform.drive.tenderdash.p2p.port', 26656 + (i * 100));
                  config.set('platform.drive.tenderdash.rpc.port', 26657 + (i * 100));

                  config.set('platform.drive.abci.log.prettyFile.path', `/tmp/drive_pretty_${nodeIndex}.log`);
                  config.set('platform.drive.abci.log.jsonFile.path', `/tmp/drive_json_${nodeIndex}.log`);
                }
              },
            }
          ));

          return new Listr(subTasks);
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
        title: 'Initialize Platform',
        task: (ctx) => initializePlatformTask(ctx.configGroup),
      },
      {
        title: 'Set default config group',
        task: (ctx, task) => {
          configFile.setDefaultGroupName(PRESET_LOCAL);

          // eslint-disable-next-line no-param-reassign
          task.output = `${PRESET_LOCAL} set as default group`;
        },
      },
    ]);
  }

  return setupLocalPresetTask;
}

module.exports = setupLocalPresetTaskFactory;
