const { Listr } = require('listr2');
const publicIp = require('public-ip');

const { PrivateKey: BlsPrivateKey } = require('bls-signatures');

const { flags: flagTypes } = require('@oclif/command');

const BaseCommand = require('../oclif/command/BaseCommand');

const MuteOneLineError = require('../oclif/errors/MuteOneLineError');

const wait = require('../util/wait');

const PRESET_TESETNET = 'testnet';
const PRESET_LOCAL = 'local';
const PRESET_EVONET = 'evonet';
const PRESETS = [PRESET_TESETNET, PRESET_EVONET, PRESET_LOCAL];

const NODE_TYPE_MASTERNODE = 'masternode';
const NODE_TYPE_FULLNODE = 'fullnode';
const NODE_TYPES = [NODE_TYPE_MASTERNODE, NODE_TYPE_FULLNODE];

class SetupCommand extends BaseCommand {
  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {DockerCompose} dockerCompose
   * @param {generateBlsKeys} generateBlsKeys
   * @param {ConfigCollection} configCollection
   * @param {initializeTenderdashNode} initializeTenderdashNode
   * @param {generateToAddressTask} generateToAddressTask
   * @param {registerMasternodeTask} registerMasternodeTask
   * @param {renderServiceTemplates} renderServiceTemplates
   * @param {writeServiceConfigs} writeServiceConfigs
   * @param {startNodeTask} startNodeTask
   * @param {initTask} initTask
   *
   * @return {Promise<void>}
   */
  async runWithDependencies(
    {
      preset,
      'node-type': nodeType,
    },
    {
      'external-ip': externalIp,
      'operator-bls-private-key': operatorBlsPrivateKey,
      update: isUpdate,
      'drive-image-build-path': driveImageBuildPath,
      'dapi-image-build-path': dapiImageBuildPath,
      verbose: isVerbose,
    },
    dockerCompose,
    generateBlsKeys,
    configCollection,
    initializeTenderdashNode,
    generateToAddressTask,
    registerMasternodeTask,
    renderServiceTemplates,
    writeServiceConfigs,
    startNodeTask,
    initTask,
  ) {
    let config;

    if (preset === PRESET_LOCAL) {
      if (nodeType === undefined) {
        // eslint-disable-next-line no-param-reassign
        nodeType = 'masternode';
      }

      if (nodeType !== NODE_TYPE_MASTERNODE) {
        throw new Error('Local development preset uses only masternode type of node');
      }
    }

    const amount = 10000;

    const tasks = new Listr([
      {
        title: 'Set configuration preset',
        task: async (ctx, task) => {
          if (ctx.preset === undefined) {
            ctx.preset = await task.prompt([
              {
                type: 'select',
                message: 'Select configuration preset',
                choices: PRESETS,
                initial: 'testnet',
              },
            ]);
          }

          configCollection.setDefaultConfigName(ctx.preset);

          config = configCollection.getDefaultConfig();

          // eslint-disable-next-line no-param-reassign
          task.output = `Selected ${config.getName()} as default config\n`;

        },
        options: { persistentOutput: true },
      },
      {
        title: 'Set node type',
        enabled: (ctx) => ctx.preset !== PRESET_LOCAL,
        task: async (ctx, task) => {
          if (ctx.nodeType === undefined) {
            ctx.nodeType = await task.prompt([
              {
                type: 'select',
                message: 'Select node type',
                choices: NODE_TYPES,
                initial: NODE_TYPE_MASTERNODE,
              },
            ]);
          }

          config.set('core.masternode.enable', ctx.nodeType === NODE_TYPE_MASTERNODE);

          // eslint-disable-next-line no-param-reassign
          task.output = `Selected ${ctx.nodeType} type\n`;
        },
        options: { persistentOutput: true },
      },
      {
        title: 'Configure external IP address',
        enabled: (ctx) => ctx.preset !== PRESET_LOCAL,
        task: async (ctx, task) => {
          if (ctx.externalIp === undefined) {
            ctx.externalIp = await task.prompt([
              {
                type: 'input',
                message: 'Enter node public IP (Enter to accept detected IP)',
                initial: () => publicIp.v4(),
              },
            ]);
          }

          config.set('externalIp', ctx.externalIp);

          // eslint-disable-next-line no-param-reassign
          task.output = `${ctx.externalIp} is set\n`;
        },
        options: { persistentOutput: true },
      },
      {
        title: 'Configure BLS private key',
        enabled: (ctx) => ctx.preset !== PRESET_LOCAL && ctx.nodeType === NODE_TYPE_MASTERNODE,
        task: async (ctx, task) => {
          if (ctx.operatorBlsPrivateKey === undefined) {
            const { privateKey: generatedPrivateKeyHex } = await generateBlsKeys();

            ctx.operatorBlsPrivateKey = await task.prompt([
              {
                type: 'input',
                message: 'Enter operator BLS private key (Enter to accept generated key)',
                initial: generatedPrivateKeyHex,
              },
            ]);
          }

          const operatorBlsPrivateKeyBuffer = Buffer.from(ctx.operatorBlsPrivateKey, 'hex');
          const privateKey = BlsPrivateKey.fromBytes(operatorBlsPrivateKeyBuffer, true);
          const publicKey = privateKey.getPublicKey();
          const publicKeyHex = Buffer.from(publicKey.serialize()).toString('hex');

          config.set('core.masternode.operator.privateKey', ctx.operatorBlsPrivateKey);

          // eslint-disable-next-line no-param-reassign
          task.output = `BLS public key: ${publicKeyHex}\nBLS private key: ${ctx.operatorBlsPrivateKey}`;
        },
        options: { persistentOutput: true },
      },
      {
        title: 'Update config',
        enabled: (ctx) => ctx.preset === PRESET_LOCAL,
        task: () => {
          const configFiles = renderServiceTemplates(config);
          writeServiceConfigs(config.getName(), configFiles);
        },
      },
      {
        title: `Generate ${amount} dash to local wallet`,
        enabled: (ctx) => ctx.preset === PRESET_LOCAL,
        task: () => generateToAddressTask(config, amount),
      },
      {
        title: 'Register masternode',
        enabled: (ctx) => ctx.preset === PRESET_LOCAL,
        task: () => registerMasternodeTask(config),
      },
      {
        title: 'Initialize Tenderdash',
        task: async (ctx) => {
          const isValidatorKeyEmpty = Object.keys(config.get('platform.drive.tenderdash.validatorKey')).length === 0;
          const isNodeKeyEmpty = Object.keys(config.get('platform.drive.tenderdash.nodeKey')).length === 0;
          const isGenesisEmpty = Object.keys(config.get('platform.drive.tenderdash.genesis')).length === 0;

          if (isValidatorKeyEmpty || isNodeKeyEmpty || isNodeKeyEmpty) {
            const [validatorKey, nodeKey, genesis] = await initializeTenderdashNode(config);

            if (isValidatorKeyEmpty) {
              config.set('platform.drive.tenderdash.validatorKey', validatorKey);
            }

            if (isNodeKeyEmpty) {
              config.set('platform.drive.tenderdash.nodeKey', nodeKey);
            }

            if (isGenesisEmpty) {
              if (ctx.preset === PRESET_LOCAL) {
                genesis.initial_core_chain_locked_height = 1000;
              }

              config.set('platform.drive.tenderdash.genesis', genesis);
            }
          }
        },
      },
      {
        title: 'Update config',
        task: () => {
          const configFiles = renderServiceTemplates(config);
          writeServiceConfigs(config.getName(), configFiles);
        },
      },
      {
        title: 'Start masternode',
        enabled: (ctx) => ctx.preset === PRESET_LOCAL,
        task: async (ctx) => startNodeTask(
          config,
          {
            driveImageBuildPath: ctx.driveImageBuildPath,
            dapiImageBuildPath: ctx.dapiImageBuildPath,
            isUpdate,
            isMinerEnabled: true,
          },
        ),
      },
      {
        title: 'Wait 20 seconds to ensure all services are running',
        enabled: (ctx) => ctx.preset === PRESET_LOCAL,
        task: async () => {
          await wait(20000);
        },
      },
      {
        title: 'Initialize Platform',
        enabled: (ctx) => ctx.preset === PRESET_LOCAL,
        task: () => initTask(config),
      },
      {
        title: 'Stop node',
        enabled: (ctx) => ctx.preset === PRESET_LOCAL,
        task: async () => dockerCompose.stop(config.toEnvs()),
      },
    ],
    {
      renderer: isVerbose ? 'verbose' : 'default',
      rendererOptions: {
        clearOutput: false,
        collapse: false,
        showSubtasks: true,
      },
    });

    try {
      await tasks.run({
        driveImageBuildPath,
        dapiImageBuildPath,
        preset,
        nodeType,
        externalIp,
        operatorBlsPrivateKey,
      });
    } catch (e) {
      throw new MuteOneLineError(e);
    }
  }
}

SetupCommand.description = `Set up node config

Set up node config
`;

SetupCommand.args = [{
  name: 'preset',
  required: false,
  description: 'Node configuration preset',
  options: PRESETS,
},
{
  name: 'node-type',
  required: false,
  description: 'Node type',
  options: NODE_TYPES,
}];

SetupCommand.flags = {
  'external-ip': flagTypes.string({ char: 'i', description: 'external ip' }),
  'operator-bls-private-key': flagTypes.string({ char: 'k', description: 'operator bls private key' }),
  update: flagTypes.boolean({ char: 'u', description: 'download updated services before start', default: false }),
  'drive-image-build-path': flagTypes.string({ description: 'drive\'s docker image build path', default: null }),
  'dapi-image-build-path': flagTypes.string({ description: 'dapi\'s docker image build path', default: null }),
  verbose: flagTypes.boolean({ char: 'v', description: 'use verbose mode for output', default: false }),

};

module.exports = SetupCommand;
