import process from 'process';
import { Flags } from '@oclif/core';
import { Listr } from 'listr2';
import chalk from 'chalk';
import archiveSamples from '../doctor/archiveSamples.js';
import unarchiveSamples from '../doctor/unarchiveSamples.js';
import ConfigBaseCommand from '../oclif/command/ConfigBaseCommand.js';
import Samples from '../doctor/Samples.js';
import MuteOneLineError from '../oclif/errors/MuteOneLineError.js';

export default class DoctorCommand extends ConfigBaseCommand {
  static description = 'Dashmate node diagnostic.  Bring your node to a doctor';

  static flags = {
    ...ConfigBaseCommand.flags,
    verbose: Flags.boolean({ char: 'v', description: 'use verbose mode for output', default: false }),
    samples: Flags.string({ char: 's', description: 'path to the samples archive', default: '' }),
  };

  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {Config} config
   * @param {analyseSamples} analyseSamples
   * @param {collectSamplesTask} collectSamplesTask
   * @param {prescriptionTask} prescriptionTask
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    {
      verbose: isVerbose,
      samples: samplesFile,
    },
    config,
    analyseSamples,
    collectSamplesTask,
    prescriptionTask,
  ) {
    const tasks = new Listr(
      [
        {
          title: 'Collecting samples',
          enabled: () => !samplesFile,
          task: async () => collectSamplesTask(config),
        },
        {
          title: 'Analyzing samples',
          task: async (ctx) => {
            ctx.prescription = analyseSamples(ctx.samples);
          },
        },
        {
          title: 'Prescription',
          task: prescriptionTask,
          options: {
            persistentOutput: true,
            bottomBar: true,
          },
        },
        {
          title: 'Archive samples',
          enabled: () => !samplesFile,
          task: async (ctx, task) => {
            const agreement = await task.prompt({
              type: 'toggle',
              name: 'confirm',
              header: chalk`  Do you want to create an archive of already collected data for further investigation?

  The archive will include:

  - System information
  - The node configuration
  - Service logs, metrics and status

  Collected data will not contain only private information which is already not available publicly.
  All sensitive data like private keys or passwords is obfuscated.

  The archive will compressed with TAR/GZIP and placed to {bold.cyanBright ${process.cwd()}}
  You can use it to analyze your node condition yourself or send it to the Dash Core Group ({underline.cyanBright support@dash.org}) in case you need help.\n`,
              message: 'Archive samples?',
              enabled: 'Yes',
              disabled: 'No',
            });

            if (!agreement) {
              task.skip();

              return;
            }

            const archivePath = process.cwd();

            await archiveSamples(ctx.samples, archivePath);

            // eslint-disable-next-line no-param-reassign
            task.output = chalk`Saved to {bold.cyanBright ${archivePath}/dashmate-report-${ctx.report.date.toISOString()}.tar.gz}`;
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
          // bottomBar: true,
          removeEmptyLines: false,
          collapse: false,
        },
      },
    );

    let samples;
    if (samplesFile) {
      samples = unarchiveSamples(samplesFile);
    } else {
      samples = new Samples();
    }

    try {
      await tasks.run({
        isVerbose,
        samples,
        problems: [],
      });
    } catch (e) {
      throw new MuteOneLineError(e);
    }
  }
}
