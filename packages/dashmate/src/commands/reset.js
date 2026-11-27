import { Listr } from 'listr2';

import { Flags } from '@oclif/core';
import ConfigBaseCommand from '../oclif/command/ConfigBaseCommand.js';
import MuteOneLineError from '../oclif/errors/MuteOneLineError.js';
import renderNodeKeyBackup from '../config/renderNodeKeyBackup.js';
import renderConfigFlag from '../util/renderConfigFlag.js';
import { PLATFORM_ONLY_RENDER_SCOPE, PRESET_MAINNET, PRESET_TESTNET } from '../constants.js';

const PLATFORM_PRESETS = [PRESET_MAINNET, PRESET_TESTNET];

export default class ResetCommand extends ConfigBaseCommand {
  // Reconfigures the node: changes configuration repeatedly while doing long,
  // partly irreversible work, so it holds the config lock for its whole run.
  static mutatesConfig = true;

  // A total reset can repair a config whose old options no longer validate.
  static shouldSkipConfigValidation({
    force, hard, platform, config,
  }) {
    if (!force || !hard || platform) {
      return false;
    }

    return ({ name, configFileData }) => (
      name === (config ?? configFileData.defaultConfigName)
    );
  }

  // A Platform-only reset changes no Core option, so Core's service files are
  // left as they are rather than written again after Platform is gone.
  static getRenderScope({ platform } = {}) {
    return platform ? PLATFORM_ONLY_RENDER_SCOPE : undefined;
  }

  static description = 'Reset node data';

  static flags = {
    ...ConfigBaseCommand.flags,
    hard: Flags.boolean({ description: 'reset config as well as services and data', default: false }),
    force: Flags.boolean({ char: 'f', description: 'skip running services check', default: false }),
    platform: Flags.boolean({ char: 'p', description: 'reset platform services and data only', default: false }),
    verbose: Flags.boolean({ char: 'v', description: 'use verbose mode for output', default: false }),
    'keep-data': Flags.boolean({ description: 'keep data', default: false }),
  };

  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {Config} config
   * @param {resetNodeTask} resetNodeTask
   * @param {HomeDir} homeDir
   *
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    {
      verbose: isVerbose,
      hard: isHardReset,
      force: isForce,
      platform: isPlatformOnlyReset,
      'keep-data': keepData,
    },
    config,
    resetNodeTask,
    homeDir,
  ) {
    // A masternode's node key is bound to its registration, and the reset
    // keeps no copy, so setting Platform up again needs the operator's backup.
    // Printed under --force too: a scripted reset never sees the confirmation.
    if (isPlatformOnlyReset && isHardReset && config.get('core.masternode.enable')
      && config.get('platform.drive.tenderdash.node.key') !== null) {
      const nodeId = config.get('platform.drive.tenderdash.node.id');
      const { backupCommand, setupCommand } = renderNodeKeyBackup(config.getName());
      const backup = `  ${backupCommand}`;

      const lines = isForce
        ? [
          `Deleting the Platform node key of ${config.getName()}, node ID ${nodeId}. The masternode's`
            + ' registration names that node ID, so setting Platform up again needs this key, and'
            + ' dashmate keeps no copy. A script that resets should back it up before, with:',
          backup,
        ]
        : [
          `This deletes the Platform node key of ${config.getName()}, node ID ${nodeId}. The`
            + ' masternode\'s registration names that node ID, so setting Platform up again needs'
            + ' this key, and dashmate keeps no copy. Unless you have a backup, answer No and back'
            + ' it up first, into a new directory only you can open:',
          backup,
        ];

      // Platform-only setup serves the mainnet and testnet presets outside a
      // group and refuses every other config, so for those the key goes back
      // where it was. `config set` would take the key on its command line.
      const restore = !config.get('group') && PLATFORM_PRESETS.includes(config.getName())
        ? [
          'After the reset, set Platform up again with:',
          `  ${setupCommand}`,
        ]
        : [
          'After the reset, put the backed-up key into platform.drive.tenderdash.node.key and its'
            + ` node ID into platform.drive.tenderdash.node.id, by editing ${homeDir.joinPath('config.json')}`
            + ' while no dashmate command runs. Then render the service files with:',
          `  dashmate config render ${renderConfigFlag(config.getName())}`,
        ];

      process.stderr.write([
        ...lines,
        'It refuses a directory or file of that name that exists. Check that the backup is not'
          + ' empty before you reset, and keep it in that directory or another one only you can open.',
        ...restore,
        '',
        '',
      ].join('\n'));
    }

    const tasks = new Listr(
      [
        {
          enabled: (ctx) => !ctx.isForce,
          task: async (ctx, task) => {
            let message;
            if (ctx.isHardReset) {
              if (ctx.keepData) {
                message = 'Are you sure you want to reset your node configuration? Data will be'
                  + ' kept.';
                if (ctx.isPlatformOnlyReset) {
                  message = 'Are you sure you want to reset platform related configuration? Data'
                    + ' will be kept';
                }
              } else {
                message = 'Are you sure you want to reset your node data and configuration?';
                if (ctx.isPlatformOnlyReset) {
                  message = 'Are you sure you want to reset platform related data and configuration?';
                }
              }
            } else if (ctx.keepData) {
              message = 'Are you sure you want to reset docker containers?';
              if (ctx.isPlatformOnlyReset) {
                message = 'Are you sure you want to reset platform related docker containers?';
              }
            } else {
              message = 'Are you sure you want to reset your node data?';
              if (ctx.isPlatformOnlyReset) {
                message = 'Are you sure you want to reset platform related data?';
              }
            }

            const agreement = await task.prompt({
              type: 'toggle',
              name: 'confirm',
              message,
              enabled: 'Yes',
              disabled: 'No',
            });

            if (!agreement) {
              throw new Error('Reset operation was declined');
            }
          },
        },
        {
          title: `Reset ${config.getName()} node`,
          task: () => resetNodeTask(config),
        },
      ],
      {
        renderer: isVerbose ? 'verbose' : 'default',
        rendererOptions: {
          showTimer: isVerbose,
          // Errors explain what to do next on several lines; truncating them
          // to the terminal width would cut the commands off.
          formatOutput: 'wrap',
          clearOutput: false,
          collapse: false,
          showSubtasks: true,
        },
      },
    );

    try {
      await tasks.run({
        isHardReset,
        isPlatformOnlyReset,
        isForce,
        isVerbose,
        keepData,
      });
    } catch (e) {
      throw new MuteOneLineError(e);
    }
  }
}
