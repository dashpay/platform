import process from 'process';
import { Flags } from '@oclif/core';
import { Listr } from 'listr2';
import chalk from 'chalk';
import Samples from '../../doctor/Samples.js';
import ConfigBaseCommand from '../../oclif/command/ConfigBaseCommand.js';
import MuteOneLineError from '../../oclif/errors/MuteOneLineError.js';

export default class ReportCommand extends ConfigBaseCommand {
  static description = `Dashmate node diagnostic report

The command collects diagnostic information and creates an obfuscated archive for further investigation`;

  static flags = {
    ...ConfigBaseCommand.flags,
    verbose: Flags.boolean({ char: 'v', description: 'use verbose mode for output', default: false }),
  };

  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {Config} config
   * @param {collectSamplesTask} collectSamplesTask
   * @param {archiveSamples} archiveSamples
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    {
      verbose: isVerbose,
    },
    config,
    collectSamplesTask,
    archiveSamples,
  ) {
    const tasks = new Listr(
      [
        {
          task: async (ctx, task) => {
            const agreement = await task.prompt({
              type: 'toggle',
              name: 'confirm',
              header: chalk`  Do you want to create an archive of diagnostic information to help with debugging?

  The archive will include:

  - System information
  - The node configuration
  - Service logs, metrics and status

  Collected data will not contain any private information which is already not available publicly.
  All sensitive data like private keys or passwords is obfuscated.

  The archive will compressed with TAR/GZIP and placed to {bold.cyanBright ${process.cwd()}}
  You can use it to analyze your node condition yourself or send it to the Dash Core Group ({underline.cyanBright support@dash.org}).\n`,
              message: 'Create an archive?',
              enabled: 'Yes',
              disabled: 'No',
            });

            if (!agreement) {
              throw new Error('Archive creation was declined');
            }
          },
        },
        {
          title: 'Collecting samples',
          task: async () => collectSamplesTask(config),
        },
        {
          title: 'Archive samples',
          task: async (ctx, task) => {
            const archivePath = process.cwd();

            await archiveSamples(ctx.samples, archivePath);

            // eslint-disable-next-line no-param-reassign
            task.output = chalk`Saved to {bold.cyanBright ${archivePath}/dashmate-report-${ctx.samples.date.toISOString()}.tar.gz}`;
          },
          options: {
            persistentOutput: true,
          },
        },
      ],
      {
        renderer: isVerbose ? 'verbose' : 'default',
        rendererOptions: {
          clearOutput: false,
          showTimer: isVerbose,
          removeEmptyLines: false,
          collapse: false,
        },
      },
    );

    try {
      await tasks.run({
        isVerbose,
        samples: new Samples(),
      });
    } catch (e) {
      throw new MuteOneLineError(e);
    }
  }
}
