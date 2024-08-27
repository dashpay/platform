import chalk from 'chalk';

export default function prescriptionTaskFactory() {
  /**
   * @param {Context} ctx
   * @param {Task} task
   * @return {Promise<void>}
   */
  async function prescriptionTask(ctx, task) {
    if (ctx.problems.length === 0) {
      // eslint-disable-next-line no-param-reassign
      task.output = chalk`The doctor didn't find any problems with your node.
  If issues still persist, please contact the Dash Core Group ({underline.cyanBright support@dash.org})`;

      return;
    }

    const problemsString = ctx.problems.map((problem, index) => {
      const indentedProblem = problem.split('\n')
        .map((line, i) => {
          if (i === 0) {
            return line;
          }

          return ' '.repeat(7) + line;
        }).join('\n');

      return `${index + 1}. ${indentedProblem}`;
    }).join('\n\n');

    const plural = ctx.problems.length > 1 ? 's' : '';
    const header = chalk`  {bold.red ${ctx.problems.length} problem${plural}} found:

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
