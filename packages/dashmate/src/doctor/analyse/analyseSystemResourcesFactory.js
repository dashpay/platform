import { SEVERITY } from '../Prescription.js';
import Problem from '../Problem.js';

/**
 * @param {verifySystemRequirements} verifySystemRequirements
 * @return {analyseSystemResources}
 */
export default function analyseSystemResourcesFactory(verifySystemRequirements) {
  /**
   * @typedef {analyseSystemResources}
   * @param {Samples} samples
   * @returns {Problem[]}
   */
  function analyseSystemResources(samples) {
    const {
      cpu,
      dockerSystemInfo,
      currentLoad,
      diskSpace,
      fsOpenFiles,
      memory,
      diskIO,
    } = samples.getSystemInfo();

    // System requirements
    const problems = verifySystemRequirements(
      {
        dockerSystemInfo,
        cpu,
        memory,
        diskSpace,
      },
      samples.getDashmateConfig().get('platform.enable'),
      {
        diskSpace: 5,
      },
    );

    // Current CPU load
    const cpuCores = dockerSystemInfo?.NCPU ?? cpu?.cores;
    if (cpuCores && currentLoad && (currentLoad.avgLoad / cpuCores) > 0.8) {
      const problem = new Problem(
        `Average system load ${currentLoad.avgLoad.toFixed(2)} is higher than normal`,
        'Consider upgrading CPUs',
        SEVERITY.LOW,
      );

      problems.push(problem);
    }

    // Free memory
    if (memory && Number.isInteger(memory.free) && memory.free > 0) {
      const memoryGb = memory.free / (1024 ** 3);
      if (memoryGb < 0.5) {
        const problem = new Problem(
          `Only ${memoryGb.toFixed(1)}GB RAM is available`,
          'Consider adding RAM',
          SEVERITY.LOW,
        );

        problems.push(problem);
      }
    }

    // Open file descriptors
    if (fsOpenFiles?.allocated && fsOpenFiles?.max) {
      const available = fsOpenFiles.max - fsOpenFiles.allocated;
      if (available < 1000) {
        const problem = new Problem(
          `${available} available file descriptors left`,
          'Please increase the maximum open file descriptor limit or stop unnecessary processes.',
          SEVERITY.HIGH,
        );

        problems.push(problem);
      }
    }

    // IO wait time
    if (diskIO?.tWaitPercent) {
      const THRESHOLD = 40;

      const maxDiskIOWaitPercent = Math.max(
        diskIO.rWaitPercent,
        diskIO.wWaitPercent,
        diskIO.tWaitPercent,
      ) * 100;

      if (maxDiskIOWaitPercent > THRESHOLD) {
        const problem = new Problem(
          `Disk IO wait time is ${maxDiskIOWaitPercent.toFixed(0)}%`,
          'Consider upgrading to faster storage',
          SEVERITY.LOW,
        );

        problems.push(problem);
      }
    }

    return problems;
  }

  return analyseSystemResources;
}
