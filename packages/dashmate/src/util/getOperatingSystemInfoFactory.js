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
      osInfo: null,
      systemData: null,
      currentLoad: null,
      diskIO: null,
      diskInfo: null,
      fsOpenFiles: null,
      inetLatency: null,
      memoryGb: null,
      memoryData: null,
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

    // Check System Data
    try {
      result.systemData = await si.system();
    } catch {
      if (process.env.DEBUG) {
        // eslint-disable-next-line no-console
        console.warn('Can\'t get System Data info');
      }
    }

    // Check OS Info
    try {
      result.osInfo = await si.osInfo();
    } catch {
      if (process.env.DEBUG) {
        // eslint-disable-next-line no-console
        console.warn('Can\'t get OS info');
      }
    }

    // Check Current Load
    try {
      result.currentLoad = await si.currentLoad();
    } catch {
      if (process.env.DEBUG) {
        // eslint-disable-next-line no-console
        console.warn('Can\'t get Current Load');
      }
    }

    // Check FS Open Files
    try {
      result.fsOpenFiles = await si.fsOpenFiles();
    } catch {
      if (process.env.DEBUG) {
        // eslint-disable-next-line no-console
        console.warn('Can\'t get FS Open Files');
      }
    }

    // Check Disk IO
    try {
      result.diskIO = await si.disksIO();
    } catch {
      if (process.env.DEBUG) {
        // eslint-disable-next-line no-console
        console.warn('Can\'t get Disk IO');
      }
    }

    // Check Inet Latency
    try {
      result.inetLatency = await si.inetLatency();
    } catch {
      if (process.env.DEBUG) {
        // eslint-disable-next-line no-console
        console.warn('Can\'t get Inet Latency');
      }
    }

    // Check swap information
    try {
      result.memoryData = await si.mem();
    } catch (e) {
      if (process.env.DEBUG) {
        // eslint-disable-next-line no-console
        console.warn(`Can't get mem info: ${e}`);
      }
    }

    if (result.memoryData) {
      result.swapTotalGb = (result.memoryData.swaptotal / (1024 ** 3)); // Convert bytes to GB
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
