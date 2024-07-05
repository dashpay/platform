import chalk from 'chalk';
import { Listr } from 'listr2';
import os from 'os';
import * as diskusage from 'diskusage';
import si from 'systeminformation';

/**
 *
 * @param {Docker} docker
 * @param {DockerCompose} dockerCompose
 * @return {verifySystemRequirementsTask}
 */
export default function verifySystemRequirementsTaskFactory(docker, dockerCompose) {
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

          // Get system info
          const systemInfo = await docker.info();

          // Check CPU cores
          const cpuCores = systemInfo.NCPU || Number.MAX_SAFE_INTEGER;

          if (cpuCores < MINIMUM_CPU_CORES) {
            warnings.push(`${cpuCores} CPU cores detected. At least ${MINIMUM_CPU_CORES} are required`);
          }

          // Check CPU frequency
          const hostCpuCores = os.cpus();

          const lessCpuFrequency = hostCpuCores
            .find((core) => (core.speed / 1000) < MINIMUM_CPU_FREQUENCY);

          if (lessCpuFrequency) {
            const cpuFrequency = (lessCpuFrequency.speed / 1000); // Convert to GHz

            warnings.push(`${cpuFrequency.toFixed(1)}GHz CPU frequency detected. At least ${MINIMUM_CPU_FREQUENCY}GHz is required`);
          }

          // Check RAM
          const memoryGb = (systemInfo.MemTotal || Number.MAX_SAFE_INTEGER)
            / (1024 ** 3); // Convert to GB

          if (memoryGb < MINIMUM_RAM) {
            warnings.push(`${memoryGb.toFixed(2)}GB RAM detected. At least ${MINIMUM_RAM}GB is required`);
          }

          // Check swap information
          const swap = await si.mem();
          const swapTotalGb = (swap.swaptotal / (1024 ** 3)); // Convert bytes to GB

          if (swapTotalGb < 2) {
            warnings.push(`Swap space is ${swapTotalGb.toFixed(2)}GB. 2GB is recommended`);
          }

          // Get disk usage info
          let diskInfo;

          try {
            diskInfo = await diskusage.check(systemInfo.DockerRootDir);
          } catch (e) {
            if (process.env.DEBUG) {
              // eslint-disable-next-line no-console
              console.error(e);
            }
          }

          if (!diskInfo) {
            try {
              diskInfo = await diskusage.check(os.platform() === 'win32' ? 'c:' : '/');
            } catch (e) {
              if (process.env.DEBUG) {
                // eslint-disable-next-line no-console
                console.error(e);
              }
            }
          }

          if (diskInfo) {
            const availableDiskSpace = diskInfo.available / (1024 ** 3); // Convert to GB

            if (availableDiskSpace < MINIMUM_DISK_SPACE) {
              warnings.push(`${availableDiskSpace.toFixed(2)}GB available disk space detected. At least ${MINIMUM_DISK_SPACE}GB is required`);
            }
          }

          if (warnings.length > 0) {
            const warningsText = warnings.map((warning) => `    - ${warning}`).join('\n');

            const header = chalk`  Minimum requirements have not been met:

{red ${warningsText}}

    Dash Platform requires more resources than the current system provides.
    Evonode rewards are paid based on block production, and resource-limited
    nodes may not be able to produce blocks quickly enough to receive reward
    payments. Upgrading system resources is recommended before proceeding.

    {bold This server may not receive Dash Platform reward payments due to its resource limitations.}\n`;

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
