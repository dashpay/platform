const { Listr } = require('listr2');

const { flags: flagTypes } = require('@oclif/command');

const BaseCommand = require('../oclif/command/BaseCommand');

const MuteOneLineError = require('../oclif/errors/MuteOneLineError');

const {
  PRESET_LOCAL,
  PRESETS,
  NODE_TYPES,
  NODE_TYPE_MASTERNODE,
  MASTERNODE_DASH_AMOUNT,
} = require('../constants');

class SetupCommand extends BaseCommand {
  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {generateBlsKeys} generateBlsKeys
   * @param {setupLocalPresetTask} setupLocalPresetTask
   * @param {setupRegularPresetTask} setupRegularPresetTask
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
      'funding-private-key': fundingPrivateKeyString,
      'node-count': nodeCount,
      'debug-logs': debugLogs,
      'miner-interval': minerInterval,
      verbose: isVerbose,
    },
    generateBlsKeys,
    setupLocalPresetTask,
    setupRegularPresetTask,
  ) {
    if (preset === PRESET_LOCAL) {
      if (nodeType === undefined) {
        // eslint-disable-next-line no-param-reassign
        nodeType = 'masternode';
      }

      if (nodeType !== NODE_TYPE_MASTERNODE) {
        throw new Error('Local development preset uses only masternode type of node');
      }
    }

    if (nodeCount !== null && (nodeCount < 3)) {
      throw new Error('node-count flag should be not less than 3');
    }

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
        },
      },
      {
        task: (ctx) => {
          if (ctx.preset === PRESET_LOCAL) {
            return setupLocalPresetTask();
          }

          return setupRegularPresetTask();
        },
      },
    ],
    {
      renderer: isVerbose ? 'verbose' : 'default',
      rendererOptions: {
        showTimer: isVerbose,
        clearOutput: false,
        collapse: false,
        showSubtasks: true,
      },
    });

    try {
      await tasks.run({
        preset,
        nodeType,
        nodeCount,
        debugLogs,
        minerInterval,
        externalIp,
        operatorBlsPrivateKey,
        fundingPrivateKeyString,
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
  'debug-logs': flagTypes.boolean({
    char: 'd', description: 'enable debug logs', allowNo: true, default: null,
  }),
  'external-ip': flagTypes.string({ char: 'i', description: 'external ip' }),
  'operator-bls-private-key': flagTypes.string({ char: 'k', description: 'operator bls private key' }),
  'funding-private-key': flagTypes.string({ char: 'p', description: `private key with more than ${MASTERNODE_DASH_AMOUNT} dash for funding collateral` }),
  'node-count': flagTypes.integer({ description: 'number of nodes to setup', default: null }),
  'miner-interval': flagTypes.string({ char: 'm', description: 'interval between blocks', default: null }),
  verbose: flagTypes.boolean({ char: 'v', description: 'use verbose mode for output', default: false }),
};

module.exports = SetupCommand;
