import si from 'systeminformation';
import * as diskusage from 'diskusage';
import os from 'os';

export default function getOperatingSystemInfoFactory(
  docker,
) {
  async function getOperatingSystemInfo() {
    const result = {
      cpuCores: null,
      hostCpu: null,
      systemInfo: null,
      diskInfo: null,
      memoryGb: null,
      swap: null,
      swapTotalGb: null,
      availableDiskSpace: null,
    };

    // Get System Info
    try {
      result.systemInfo = await docker.info();
    } catch (e) {
      if (process.env.DEBUG) {
        // eslint-disable-next-line no-console
        console.warn(`Can't get docker info: ${e}`);
      }
    }

    if (result.systemInfo) {
      if (Number.isInteger(result.systemInfo.NCPU)) {
        // Check CPU cores
        result.cpuCores = result.systemInfo.NCPU;
      } else {
        // eslint-disable-next-line no-console
        console.warn('Can\'t get NCPU from docker info');
      }

      // Check RAM
      if (Number.isInteger(result.systemInfo.MemTotal)) {
        result.memoryGb = result.systemInfo.MemTotal / (1024 ** 3); // Convert to GB
      } else {
        // eslint-disable-next-line no-console
        console.warn('Can\'t get MemTotal from docker info');
      }
    }

    // Check CPU frequency
    try {
      result.hostCpu = await si.cpu();
    } catch {
      if (process.env.DEBUG) {
        // eslint-disable-next-line no-console
        console.warn('Can\'t get CPU info');
      }
    }
    // Check swap information
    try {
      result.swap = await si.mem();
    } catch (e) {
      if (process.env.DEBUG) {
        // eslint-disable-next-line no-console
        console.warn(`Can't get swap info: ${e}`);
      }
    }

    if (result.swap) {
      result.swapTotalGb = (result.swap.swaptotal / (1024 ** 3)); // Convert bytes to GB
    }

    // Get disk usage info
    if (result.systemInfo) {
      try {
        result.diskInfo = await diskusage.check(result.systemInfo.DockerRootDir);
      } catch (e) {
        if (process.env.DEBUG) {
          // eslint-disable-next-line no-console
          console.warn(`Can't get disk usage for '${result.systemInfo.DockerRootDir}': ${e}`);
        }
      }
    }

    if (!result.diskInfo) {
      try {
        result.diskInfo = await diskusage.check(os.platform() === 'win32' ? 'c:' : '/');
      } catch (e) {
        if (process.env.DEBUG) {
          // eslint-disable-next-line no-console
          console.warn(`Can't get disk usage for root directory: ${e}`);
        }
      }
    }

    if (result.diskInfo) {
      result.availableDiskSpace = result.diskInfo.available / (1024 ** 3); // Convert to GB
    }

    return result;
  }
  return getOperatingSystemInfo;
}
