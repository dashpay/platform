import chalk from 'chalk';
import { Listr } from 'listr2';

/**
 *
 * @param {Docker} docker
 * @param {DockerCompose} dockerCompose
 * @param {getOperatingSystemInfo} getOperatingSystemInfo
 * @return {verifySystemRequirementsTask}
 */
export default function verifySystemRequirementsTaskFactory(
  docker,
  dockerCompose,
  getOperatingSystemInfo,
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

          const MINIMUM_CPU_CORES = ctx.isHP ? 4 : 2;
          const MINIMUM_CPU_FREQUENCY = 2.4; // GHz
          const MINIMUM_RAM = ctx.isHP ? 8 : 4; // GB
          const MINIMUM_DISK_SPACE = ctx.isHP ? 200 : 100; // GB

          const warnings = [];

          const {
            dockerSystemInfo, cpu, memory, diskSpace,
          } = await getOperatingSystemInfo();

          if (dockerSystemInfo) {
            if (Number.isInteger(dockerSystemInfo.NCPU)) {
              // Check CPU cores
              const cpuCores = dockerSystemInfo.NCPU;

              if (cpuCores < MINIMUM_CPU_CORES) {
                warnings.push(`${cpuCores} CPU cores detected. At least ${MINIMUM_CPU_CORES} are required`);
              }
            } else {
              // eslint-disable-next-line no-console
              console.warn('Can\'t get NCPU from docker info');
            }

            // Check RAM
            if (Number.isInteger(dockerSystemInfo.MemTotal)) {
              const memoryGb = dockerSystemInfo.MemTotal / (1024 ** 3); // Convert to GB

              if (memoryGb < MINIMUM_RAM) {
                warnings.push(`${memoryGb.toFixed(2)}GB RAM detected. At least ${MINIMUM_RAM}GB is required`);
              }
            } else {
              // eslint-disable-next-line no-console
              console.warn('Can\'t get MemTotal from docker info');
            }
          }

          // Check CPU frequency
          if (cpu) {
            if (cpu.speed === 0) {
              if (process.env.DEBUG) {
                // eslint-disable-next-line no-console
                console.warn('Can\'t get CPU frequency');
              }
            } else if (cpu.speed < MINIMUM_CPU_FREQUENCY) {
              warnings.push(`${cpu.speed.toFixed(1)}GHz CPU frequency detected. At least ${MINIMUM_CPU_FREQUENCY}GHz is required`);
            }
          }

          // Check swap information
          if (memory) {
            const swapTotalGb = (memory.swaptotal / (1024 ** 3)); // Convert bytes to GB

            if (swapTotalGb < 2) {
              warnings.push(`Swap space is ${swapTotalGb.toFixed(2)}GB. 2GB is recommended`);
            }
          }

          // Get disk usage info
          if (diskSpace) {
            const availableDiskSpace = diskSpace.available / (1024 ** 3); // Convert to GB

            if (availableDiskSpace < MINIMUM_DISK_SPACE) {
              warnings.push(`${availableDiskSpace.toFixed(2)}GB available disk space detected. At least ${MINIMUM_DISK_SPACE}GB is required`);
            }
          }

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

          if (warnings.length > 0) {
            const warningsText = warnings.map((warning) => `    - ${warning}`).join('\n');

            const header = chalk`  Minimum requirements have not been met:

{red ${warningsText}}

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
