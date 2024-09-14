import chalk from 'chalk';
import { Listr } from 'listr2';
import { SEVERITY } from '../../../../doctor/Prescription.js';

/**
 *
 * @param {Docker} docker
 * @param {DockerCompose} dockerCompose
 * @param {getOperatingSystemInfo} getOperatingSystemInfo
 * @param {verifySystemRequirements} verifySystemRequirements
 * @return {verifySystemRequirementsTask}
 */
export default function verifySystemRequirementsTaskFactory(
  docker,
  dockerCompose,
  getOperatingSystemInfo,
  verifySystemRequirements,
) {
  /**
   * @typedef {function} verifySystemRequirementsTask
   * @returns {Listr}
   */
  async function verifySystemRequirementsTask() {
    return new Listr([
      {
        title: 'System requirements',
        task: async (ctx, task) => {
          await dockerCompose.throwErrorIfNotInstalled();

          const systemInfo = await getOperatingSystemInfo();

          const problems = verifySystemRequirements(systemInfo, ctx.isHP);

          let message = '';
          if (ctx.isHP) {
            message = chalk`Dash Platform requires more resources than the current system provides.
    Evonode rewards are paid based on block production, and resource-limited
    nodes may not be able to produce blocks quickly enough to receive reward
    payments. Upgrading system resources is recommended before proceeding.

    {bold This node may not receive Dash Platform reward payments due to its resource limitations.}`;
          } else {
            message = `Limited system resources may impact the performance of the node.
    The node might not provide required services to the network in time and will get PoSe banned.
    PoSe banned node aren't receiving masternode rewards.
    Upgrading system resources is recommended before proceeding.`;
          }

          if (problems.length > 0) {
            const problemsText = problems
              .map((p) => `    - ${p.getDescription()}`).join('\n');

            const header = chalk`  Minimum requirements have not been met:

{red ${problemsText}}

    ${message}\n`;

            // This option is used for tests
            if (ctx.acceptUnmetSystemRequirements) {
              // eslint-disable-next-line no-console
              console.warn(header);
            } else {
              const proceed = await task.prompt({
                type: 'toggle',
                header,
                message: ' Are you sure you want to proceed?',
                enabled: 'Yes',
                disabled: 'No',
                initial: false,
              });

              if (!proceed) {
                throw new Error('System requirements have not been met');
              } else {
                // eslint-disable-next-line no-param-reassign
                task.output = chalk`{yellow System requirements have not been met.}`;
              }
            }
          }
        },
        options: {
          persistentOutput: true,
        },
      },
    ]);
  }

  return verifySystemRequirementsTask;
}
