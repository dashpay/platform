import chalk from 'chalk';
import { SEVERITY } from '../../../doctor/Prescription.js';

export default function prescriptionTaskFactory() {
  /**
   * @param {Context} ctx
   * @param {Task} task
   * @return {Promise<void>}
   */
  async function prescriptionTask(ctx, task) {
    const problems = ctx.prescription.getOrderedProblems();
    if (problems.length === 0) {
      // eslint-disable-next-line no-param-reassign
      task.output = chalk`The doctor didn't find any problems with your node.
  If issues still persist, please contact the Dash Core Group ({underline.cyanBright support@dash.org})`;

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

    const header = chalk`  ${problemsCount} found:

${problemsString}

    You can try to fix the problems or contact the Dash Core Group ({underline.cyanBright support@dash.org})
    `;

    await task.prompt({
      type: 'confirm',
      header,
      message: 'Press any key to continue...',
      default: ' ',
      separator: () => '',
      format: () => '',
      initial: true,
      isTrue: () => true,
    });

    // eslint-disable-next-line no-param-reassign
    task.output = `\n${header}`;
  }

  return prescriptionTask;
}
