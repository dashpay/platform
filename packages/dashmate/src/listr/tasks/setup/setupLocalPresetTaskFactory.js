const { Listr } = require('listr2');

const path = require('path');

const {
  PRESET_LOCAL,
  HOME_DIR_PATH,
  SSL_PROVIDERS,
} = require('../../../constants');
const generateTenderdashNodeKey = require('../../../tenderdash/generateTenderdashNodeKey');
const deriveTenderdashNodeId = require('../../../tenderdash/deriveTenderdashNodeId');

/**
 * @param {ConfigFile} configFile
 * @param {configureCoreTask} configureCoreTask
 * @param {configureTenderdashTask} configureTenderdashTask
 * @param {obtainSelfSignedCertificateTask} obtainSelfSignedCertificateTask
 * @param {resolveDockerHostIp} resolveDockerHostIp
 * @param {configFileRepository} configFileRepository
 * @param {generateSystemDataContractKeysTask} generateSystemDataContractKeysTask
 */
function setupLocalPresetTaskFactory(
  configFile,
  configureCoreTask,
  obtainSelfSignedCertificateTask,
  configureTenderdashTask,
  resolveDockerHostIp,
  configFileRepository,
  generateSystemDataContractKeysTask,
) {
  /**
   * @typedef {setupLocalPresetTask}
   * @return {Listr}
   */
  function setupLocalPresetTask() {
    return new Listr([
      {
        title: 'Set the number of nodes',
        enabled: (ctx) => ctx.nodeCount === undefined,
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

          const network = ctx.configGroup[0].get('network');

          const subTasks = ctx.configGroup.map((config, i) => (
            {
              title: `Create ${config.getName()} config`,
              // eslint-disable-next-line consistent-return
              task: () => {
                const nodeIndex = i + 1;

                config.set('group', 'local');
                config.set('core.p2p.port', config.get('core.p2p.port') + (i * 100));
                config.set('core.rpc.port', config.get('core.rpc.port') + (i * 100));
                config.set('externalIp', hostDockerInternalIp);

                config.set('docker.network.subnet', `172.24.${nodeIndex}.0/24`);

                // Setup Core debug logs
                if (ctx.debugLogs) {
                  config.set('core.debug', 1);
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
                  config.set('platform.enable', false);
                } else {
                  config.set('description', `local node #${nodeIndex}`);

                  const key = generateTenderdashNodeKey();
                  const id = deriveTenderdashNodeId(key);

                  config.set('platform.drive.tenderdash.node.id', id);
                  config.set('platform.drive.tenderdash.node.key', key);

                  config.set('platform.dapi.envoy.http.port', config.get('platform.dapi.envoy.http.port') + (i * 100));
                  config.set('platform.drive.tenderdash.p2p.port', config.get('platform.drive.tenderdash.p2p.port') + (i * 100));
                  config.set('platform.drive.tenderdash.rpc.port', config.get('platform.drive.tenderdash.rpc.port') + (i * 100));
                  config.set('platform.drive.tenderdash.moniker', config.name);

                  // Setup logs
                  if (ctx.debugLogs) {
                    config.set('platform.drive.abci.log.stdout.level', 'trace');
                    config.set('platform.drive.abci.log.prettyFile.level', 'trace');

                    config.set('platform.drive.tenderdash.log.level', 'debug');
                  }

                  if (!config.get('platform.drive.abci.log.prettyFile.path')) {
                    const drivePrettyLogFile = path.join(HOME_DIR_PATH, 'logs', config.getName(), 'drive_pretty.log');
                    config.set('platform.drive.abci.log.prettyFile.path', drivePrettyLogFile);
                  }

                  if (!config.get('platform.drive.abci.log.jsonFile.path')) {
                    const driveJsonLogFile = path.join(HOME_DIR_PATH, 'logs', config.getName(), 'drive_json.log');
                    config.set('platform.drive.abci.log.jsonFile.path', driveJsonLogFile);
                  }

                  config.set('dashmate.helper.api.port', config.get('dashmate.helper.api.port') + (i * 100));

                  return generateSystemDataContractKeysTask(config, network);
                }
              },
              options: {
                persistentOutput: true,
              },
            }
          ));

          subTasks.push({
            title: 'Save configs',
            task: async () => {
              configFile.setDefaultGroupName(PRESET_LOCAL);

              // Persist configs
              configFileRepository.write(configFile);
            },
          });

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

          const subTasks = platformConfigs.map((config) => {
            config.set('platform.dapi.envoy.ssl.provider', SSL_PROVIDERS.SELF_SIGNED);

            return {
              title: `Generate certificate for ${config.getName()}`,
              task: async () => obtainSelfSignedCertificateTask(config),
            };
          });

          return new Listr(subTasks);
        },
      },
    ]);
  }

  return setupLocalPresetTask;
}

module.exports = setupLocalPresetTaskFactory;
