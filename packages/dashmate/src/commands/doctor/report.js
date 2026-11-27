import os from 'os';
import process from 'process';
import { Flags } from '@oclif/core';
import { Listr } from 'listr2';
import chalk from 'chalk';
import Samples from '../../doctor/Samples.js';
import createLegacyReportCleanupTasks from '../../listr/tasks/doctor/legacyReportCleanupTasks.js';
import ConfigBaseCommand from '../../oclif/command/ConfigBaseCommand.js';
import MuteOneLineError from '../../oclif/errors/MuteOneLineError.js';

export default class ReportCommand extends ConfigBaseCommand {
  static description = `Dashmate node diagnostic report

The command collects diagnostic information and creates an archive, with the node's secrets masked, for further investigation`;

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

  Masked: secrets in the current node configuration (private keys, passwords,
  API keys, build arguments, command-line option values, credentials in URLs),
  the values of container environment variables, your home directory, and your
  user name in logs and paths. The commands containers run, what their health
  checks print, and the node description and the Let's Encrypt contact address
  in the config copy are left out; a log can still quote the contact address.
  Copies of those config secrets in logs, RPC responses, messages and paths are
  masked when they are 8 characters or longer. The values doctor decides on,
  such as certificate status codes, dates and fingerprints, are kept as they
  are.

  The archive can still contain other sensitive text, such as earlier passwords,
  short values, or a secret spelled like one of those kept values, so review it
  before you share it. The rest is diagnostic data
  about your node, including its addresses, host details, certificate settings
  and logs, and is shared with whoever you send the archive to.

  The archive will be compressed with TAR/GZIP and placed in {bold.cyanBright ${process.cwd()}}
  You can use it to analyze the node's condition yourself or send it to the Dash Core Group support team ({underline.cyanBright support@dash.org}) for help.\n`,
              message: 'Create an archive?',
              enabled: 'Yes',
              disabled: 'No',
            });

            if (!agreement) {
              throw new Error('Archive creation was declined');
            }
          },
        },
        // Only looked for once the operator has agreed to a report, so a run
        // they decline touches nothing
        ...createLegacyReportCleanupTasks(os.tmpdir()),
        {
          title: 'Collecting samples',
          task: async () => collectSamplesTask(config),
        },
        {
          title: 'Creating archive',
          task: async (ctx, task) => {
            const archivePath = process.cwd();

            const archiveFilePath = await archiveSamples(ctx.samples, archivePath, config);

            // eslint-disable-next-line no-param-reassign
            task.output = chalk`Saved to {bold.cyanBright ${archiveFilePath}}`;
          },
          options: {
            persistentOutput: true,
          },
        },
      ],
      {
        renderer: isVerbose ? 'verbose' : 'default',
        rendererOptions: {
          // Wrapped rather than cut at the terminal's width: what the report
          // says includes paths the operator has to act on
          formatOutput: 'wrap',
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
