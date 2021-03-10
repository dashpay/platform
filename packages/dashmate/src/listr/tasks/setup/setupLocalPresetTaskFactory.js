const { Listr } = require('listr2');
const { PRESET_LOCAL } = require('../../../constants');

/**
 * @param {ConfigFile} configFile
 * @param {configureCoreTask} configureCoreTask
 * @param {configureTenderdashTask} configureTenderdashTask
 * @param {configureTenderdashTask} initializePlatformTask
 */
function setupLocalPresetTaskFactory(
  configFile,
  configureCoreTask,
  configureTenderdashTask,
  initializePlatformTask,
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
        title: 'Create local group configs',
        task: (ctx) => {
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

          const subTasks = ctx.configGroup.map((config, i) => (
            {
              title: `Create ${config.getName()} config`,
              task: () => {
                const nodeIndex = i + 1;

                config.set('group', 'local');
                config.set('core.p2p.port', 20001 + (i * 100));
                config.set('core.rpc.port', 20002 + (i * 100));
                config.set('externalIp', '127.0.0.1');

                if (config.getName() === 'local_seed') {
                  config.set('description', 'seed node for local network');

                  config.set('compose.file', 'docker-compose.yml');
                  config.set('core.masternode.enable', false);
                } else {
                  config.set('description', `local node #${nodeIndex}`);

                  config.set('platform.dapi.nginx.http.port', 3000 + (i * 100));
                  config.set('platform.dapi.nginx.grpc.port', 3010 + (i * 100));
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
