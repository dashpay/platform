/**
 * @return {verifySystemRequirements}
 */
function verifySystemRequirementsFactory() {
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
   * @returns {Object}
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

    const warnings = {};

    // CPU cores
    const cpuCores = dockerSystemInfo?.NCPU ?? cpu?.cores;

    if (cpuCores) {
      if (cpuCores < MINIMUM_CPU_CORES) {
        warnings.cpuCores = `${cpuCores} CPU cores detected. At least ${MINIMUM_CPU_CORES} are required`;
      }
    } else if (process.env.DEBUG) {
      // eslint-disable-next-line no-console
      console.warn('Can\'t get CPU cores information');
    }

    // Memory
    const totalMemory = dockerSystemInfo?.MemTotal ?? memory?.total;

    if (totalMemory) {
      const totalMemoryGb = totalMemory / (1024 ** 3); // Convert to GB

      if (totalMemoryGb < MINIMUM_RAM) {
        warnings.memory = `${totalMemoryGb.toFixed(2)}GB RAM detected. At least ${MINIMUM_RAM}GB is required`;
      }
    } else if (process.env.DEBUG) {
      // eslint-disable-next-line no-console
      console.warn('Can\'t get memory information');
    }

    // CPU speed
    if (cpu && cpu.speed !== 0) {
      if (cpu.speed < MINIMUM_CPU_FREQUENCY) {
        warnings.cpuSpeed = `${cpu.speed.toFixed(1)}GHz CPU frequency detected. At least ${MINIMUM_CPU_FREQUENCY}GHz is required`;
      }
    } else if (process.env.DEBUG) {
      // eslint-disable-next-line no-console
      console.warn('Can\'t get CPU frequency');
    }

    // Check swap information
    if (memory) {
      const swapTotalGb = (memory.swaptotal / (1024 ** 3)); // Convert bytes to GB

      if (swapTotalGb < 2) {
        warnings.swap = `Swap space is ${swapTotalGb.toFixed(2)}GB. 2GB is recommended`;
      }
    }

    // Get disk usage info
    if (diskSpace) {
      const availableDiskSpace = diskSpace.available / (1024 ** 3); // Convert to GB

      if (availableDiskSpace < MINIMUM_DISK_SPACE) {
        warnings.diskSpace = `${availableDiskSpace.toFixed(2)}GB available disk space detected. At least ${MINIMUM_DISK_SPACE}GB is required`;
      }
    }

    return warnings;
  }

  return verifySystemRequirements;
}

module.exports = verifySystemRequirementsFactory;
