import process from 'process';
import { Flags } from '@oclif/core';
import { Listr } from 'listr2';
import chalk from 'chalk';
import { SEVERITY } from '../../doctor/Prescription.js';
import ConfigBaseCommand from '../../oclif/command/ConfigBaseCommand.js';
import Samples from '../../doctor/Samples.js';
import MuteOneLineError from '../../oclif/errors/MuteOneLineError.js';

export default class DoctorCommand extends ConfigBaseCommand {
  static description = 'Dashmate node diagnostic. Bring your node to a doctor';

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
   * @param {unarchiveSamples} unarchiveSamples
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
    unarchiveSamples,
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

    let samples;
    if (samplesFile) {
      samples = await unarchiveSamples(samplesFile);
    } else {
      samples = new Samples();
    }

    let ctx;
    try {
      ctx = await tasks.run({
        isVerbose,
        samples,
      });
    } catch (e) {
      throw new MuteOneLineError(e);
    }

    const problems = ctx.prescription.getOrderedProblems();
    if (problems.length === 0) {
      // eslint-disable-next-line no-console
      console.log(chalk`\n  The doctor didn't find any problems with your node.

  If issues still persist, please use {bold.cyanBright dashmate doctor report} to create an archive
  of already collected data for further investigation.

  You can use it to analyze your node condition yourself or send it to the Dash Core Group ({underline.cyanBright support@dash.org}).`);

      return;
    }

    const problemsString = problems.map((problem, index) => {
      let numberedDescription = `${index + 1}. ${problem.getDescription()}`;
      if (problem.getSeverity() === SEVERITY.HIGH) {
        numberedDescription = chalk.red(numberedDescription);
      } else if (problem.getSeverity() === SEVERITY.MEDIUM) {
        numberedDescription = chalk.yellow(numberedDescription);
      }

      const indentedDescription = numberedDescription.split('\n')
        .map((line, i) => {
          let size = 5;
          if (i === 0) {
            size = 3;
          }

          return ' '.repeat(size) + line;
        }).join('\n');

      const indentedSolution = problem.getSolution().split('\n')
        .map((line) => ' '.repeat(6) + line).join('\n');

      return `${indentedDescription}\n\n${indentedSolution}`;
    }).join('\n\n');

    const plural = problems.length > 1 ? 's' : '';

    const severity = ctx.prescription.getSeverity();

    let problemsCount = `${problems.length} problem${plural}`;
    if (severity === SEVERITY.HIGH) {
      problemsCount = chalk.red(problemsCount);
    } else if (severity === SEVERITY.MEDIUM) {
      problemsCount = chalk.yellow(problemsCount);
    }

    const prescriptionString = chalk`\n  ${problemsCount} found:

${problemsString}


  Use {bold.cyanBright dashmate doctor report} to create an archive
  of already collected data for further investigation.

  You can use it to analyze your node condition yourself or send it to the Dash Core Group ({underline.cyanBright support@dash.org}).`;

    // eslint-disable-next-line no-console
    console.log(prescriptionString);

    if (severity === SEVERITY.HIGH) {
      process.exitCode = 1;
    }
  }
}
