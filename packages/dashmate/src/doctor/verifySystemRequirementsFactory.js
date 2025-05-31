import { SEVERITY } from './Prescription.js';
import Problem from './Problem.js';

/**
 * @return {verifySystemRequirements}
 */
export default function verifySystemRequirementsFactory() {
  /**
   * @typedef {Function} verifySystemRequirements
   * @param {Object} systemInfo
   * @param {Object} systemInfo.dockerSystemInfo
   * @param {Object} systemInfo.cpu
   * @param {Object} systemInfo.memory
   * @param {Object} systemInfo.diskSpace
   * @param {boolean} isHP
   * @param {Object} [overrideRequirements]
   * @param {Number} [overrideRequirements.diskSpace]
   * @returns {Problem[]}
   */
  function verifySystemRequirements(
    {
      dockerSystemInfo,
      cpu,
      memory,
      diskSpace,
    },
    isHP,
    overrideRequirements = {},
  ) {
    const MINIMUM_CPU_CORES = isHP ? 4 : 2;
    const MINIMUM_CPU_FREQUENCY = 2.4; // GHz
    const MINIMUM_RAM = isHP ? 8 : 4; // GB
    const MINIMUM_DISK_SPACE = overrideRequirements.diskSpace ?? (isHP ? 200 : 100); // GB

    const problems = [];

    // CPU cores
    const cpuCores = dockerSystemInfo?.NCPU ?? cpu?.cores;

    if (Number.isInteger(cpuCores)) {
      if (cpuCores < MINIMUM_CPU_CORES) {
        const problem = new Problem(
          `${cpuCores} CPU cores detected. At least ${MINIMUM_CPU_CORES} are required`,
          `Consider upgrading CPUs to make sure the node can provide timely responses
for required network services and avoid Proof-of-Service bans`,
          SEVERITY.MEDIUM,
        );

        problems.push(problem);
      }
    } else if (process.env.DEBUG) {
      // eslint-disable-next-line no-console
      console.warn('Can\'t get CPU core information');
    }

    // Memory
    const totalMemory = dockerSystemInfo?.MemTotal ?? memory?.total;

    if (Number.isInteger(totalMemory)) {
      const totalMemoryGb = totalMemory / (1024 ** 3); // Convert to GB

      if (totalMemoryGb < MINIMUM_RAM) {
        const problem = new Problem(
          `${totalMemoryGb.toFixed(2)}GB RAM detected. At least ${MINIMUM_RAM}GB is required`,
          `Consider upgrading RAM to make sure the node can provide timely responses
for required network services and avoid Proof-of-Service bans`,
          SEVERITY.MEDIUM,
        );

        problems.push(problem);
      }
    } else if (process.env.DEBUG) {
      // eslint-disable-next-line no-console
      console.warn('Can\'t get memory information');
    }

    // CPU speed
    if (cpu && Number.isFinite(cpu.speed) && cpu.speed !== 0) {
      if (cpu.speed < MINIMUM_CPU_FREQUENCY) {
        const problem = new Problem(
          `${cpu.speed.toFixed(1)}GHz CPU frequency detected. At least ${MINIMUM_CPU_FREQUENCY}GHz is required`,
          `Consider upgrading CPUs to make sure the node can provide timely responses
for required network services and avoid Proof-of-Service bans`,
          SEVERITY.MEDIUM,
        );

        problems.push(problem);
      }
    } else if (process.env.DEBUG) {
      // eslint-disable-next-line no-console
      console.warn('Can\'t get CPU frequency');
    }

    // Check swap information
    if (memory && Number.isInteger(memory.swaptotal)) {
      const swapTotalGb = (memory.swaptotal / (1024 ** 3)); // Convert bytes to GB

      if (swapTotalGb < 2) {
        const problem = new Problem(
          `Swap space is ${swapTotalGb.toFixed(2)}GB. 2GB is recommended`,
          `Consider enabling SWAP to make sure the node can provide timely responses
for required network services and avoid Proof-of-Service bans`,
          SEVERITY.LOW,
        );

        problems.push(problem);
      }
    }

    // Get disk usage info
    if (diskSpace) {
      const availableDiskSpace = diskSpace.free / (1024 ** 3); // Convert to GB

      if (availableDiskSpace < MINIMUM_DISK_SPACE) {
        const problem = new Problem(
          `${availableDiskSpace.toFixed(2)}GB of available disk space detected. At least ${MINIMUM_DISK_SPACE}GB is required`,
          `Consider increasing disk space to make sure the node can provide timely responses
for required network services and avoid Proof-of-Service bans`,
          MINIMUM_DISK_SPACE - availableDiskSpace < 5 ? SEVERITY.HIGH : SEVERITY.MEDIUM,
        );

        problems.push(problem);
      }
    }

    return problems;
  }

  return verifySystemRequirements;
}
