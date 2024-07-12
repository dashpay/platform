import { Listr } from 'listr2';

import { Args, Flags } from '@oclif/core';

import chalk from 'chalk';
import BaseCommand from '../oclif/command/BaseCommand.js';
import MuteOneLineError from '../oclif/errors/MuteOneLineError.js';

import {
  PRESET_LOCAL,
  PRESET_MAINNET,
  PRESETS,
} from '../constants.js';

export default class SetupCommand extends BaseCommand {
  static description = 'Set up a new Dash node';

  static args = {
    preset: Args.string(
      {
        name: 'preset',
        required: false,
        description: 'Node configuration preset',
        options: Object.values(PRESETS),
      },
    ),
  };

  static flags = {
    'debug-logs': Flags.boolean({ char: 'd', description: 'enable debug logs', allowNo: true }),
    'node-count': Flags.integer({ char: 'c', description: 'number of nodes to setup' }),
    'miner-interval': Flags.string({ char: 'm', description: 'interval between blocks' }),

    verbose: Flags.boolean({ char: 'v', description: 'use verbose mode for output', default: false }),
  };

  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {ConfigFile} configFile
   * @param {setupLocalPresetTask} setupLocalPresetTask
   * @param {setupRegularPresetTask} setupRegularPresetTask
   * @param {DockerCompose} dockerCompose
   * @param {DefaultConfigs} defaultConfigs
   * @return {Promise<void>}
   */
  async runWithDependencies(
    {
      preset,
    },
    {
      'node-count': nodeCount,
      'debug-logs': debugLogs,
      'miner-interval': minerInterval,
      verbose: isVerbose,
    },
    configFile,
    setupLocalPresetTask,
    setupRegularPresetTask,
    dockerCompose,
    defaultConfigs,
  ) {
    if (nodeCount !== null && (nodeCount < 3)) {
      throw new Error('node-count flag should be not less than 3');
    }

    const tasks = new Listr(
      [
        {
          title: 'Configuration preset',
          task: async (ctx, task) => {
            if (ctx.preset === undefined) {
              ctx.preset = await task.prompt([
                {
                  type: 'select',
                  header: `  Dashmate provides three default configuration presets:

    mainnet - Run a node connected to the Dash main network
    testnet - Run a node connected to the Dash test network
    local   - Run a full network environment on your machine for local development\n`,
                  message: 'Select preset',
                  choices: Object.values(PRESETS),
                  initial: PRESET_MAINNET,
                },
              ]);
            }

            let isAlreadyConfigured;
            if (ctx.preset === PRESET_LOCAL) {
              isAlreadyConfigured = configFile.isGroupExists(ctx.preset);
            } else {
              const defaultConfig = defaultConfigs.get(ctx.preset);

              isAlreadyConfigured = !configFile.getConfig(ctx.preset).isEqual(defaultConfig);
            }

            if (isAlreadyConfigured) {
              const resetCommand = ctx.preset === PRESET_LOCAL
                ? `dashmate group reset --group ${ctx.preset} --hard` : `dashmate reset --config ${ctx.preset} --hard`;

              // eslint-disable-next-line no-param-reassign
              task.output = chalk`Preset {bold ${ctx.preset}} already configured.

  To set up a node with this preset from scratch use {bold.cyanBright ${resetCommand}}.
  Previous data and configuration for this preset will be lost.

  If you want to keep the existing data and configuration, please use the {bold.cyanBright dashmate config create}
  command to create a new configuration for this preset.`;
              throw new Error(`Preset ${ctx.preset} already configured`);
            } else {
            // eslint-disable-next-line no-param-reassign
              task.output = ctx.preset;
            }
          },
          options: {
            persistentOutput: true,
            showErrorMessage: false,
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
        concurrent: false,
        renderer: isVerbose ? 'verbose' : 'default',
        rendererOptions: {
          showTimer: isVerbose,
          clearOutput: false,
          collapse: false,
          showSubtasks: true,
          removeEmptyLines: false,
        },
      },
    );

    if (!isVerbose) { // TODO: We need to print it only with default renderer
      // eslint-disable-next-line import/extensions
      const { begoo } = await import('begoo/index.js'); // don't remove index!

      const welcomeText = begoo(
        chalk`Hello! I'm your {bold.cyanBright Dash} mate!

I will assist you with setting up a Dash node on mainnet or testnet. I can also help you set up a development network on your local system.`,
        { maxLength: 45 },
      );

      // eslint-disable-next-line no-console
      console.log(welcomeText);
    }

    try {
      await tasks.run({
        preset,
        nodeCount,
        debugLogs,
        minerInterval,
        isVerbose,
      });
    } catch (e) {
      throw new MuteOneLineError(e);
    }
  }
}
