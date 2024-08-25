import si from 'systeminformation';
import * as diskusage from 'diskusage';
import os from 'os';
import obfuscateObjectRecursive from './obfuscateObjectRecursive.js';
import hideString from './hideString.js';

export default function getOperatingSystemInfoFactory(
  docker,
) {
  async function getOperatingSystemInfo() {
    const result = {
      cpu: null,
      osInfo: null,
      dockerSystemInfo: null,
      currentLoad: null,
      diskIO: null,
      diskSpace: null,
      fsOpenFiles: null,
      inetLatency: null,
      memory: null,
    };

    // Get System Info
    try {
      result.dockerSystemInfo = await docker.info();

      // hide user
      obfuscateObjectRecursive(result.dockerSystemInfo, (field, value) => (typeof value === 'string' ? value.replaceAll(
        process.env.USER,
        hideString(process.env.USER),
      ) : value));
    } catch (e) {
      if (process.env.DEBUG) {
        // eslint-disable-next-line no-console
        console.warn(`Can't get docker info: ${e}`);
      }
    }

    // Check CPU frequency
    try {
      const {
        manufacturer, brand, vendor, family, model,
        speed, speedMin, speedMax,
        governor, cores, physicalCores, performanceCores,
        efficiencyCores, processors,
      } = await si.cpu();

      result.cpu = {
        manufacturer,
        brand,
        vendor,
        family,
        model,
        speed,
        speedMin,
        speedMax,
        governor,
        cores,
        physicalCores,
        performanceCores,
        efficiencyCores,
        processors,
      };
    } catch {
      if (process.env.DEBUG) {
        // eslint-disable-next-line no-console
        console.warn('Can\'t get CPU info');
      }
    }

    // Check OS Info
    try {
      const {
        platform, distro, release, codename, kernel, arch, codepage, build, servicepack,
      } = await si.osInfo();

      result.osInfo = {
        platform,
        distro,
        release,
        codename,
        kernel,
        arch,
        codepage,
        build,
        servicepack,
      };
    } catch {
      if (process.env.DEBUG) {
        // eslint-disable-next-line no-console
        console.warn('Can\'t get OS info');
      }
    }

    // Check Current Load
    try {
      const {
        avgLoad,
        currentLoad,
      } = await si.currentLoad();

      result.currentLoad = {
        avgLoad,
        currentLoad,
      };
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
      result.memory = await si.mem();
    } catch (e) {
      if (process.env.DEBUG) {
        // eslint-disable-next-line no-console
        console.warn(`Can't get mem info: ${e}`);
      }
    }

    // Get disk usage info
    if (result.dockerSystemInfo) {
      try {
        result.diskSpace = await diskusage.check(result.dockerSystemInfo.DockerRootDir);
      } catch (e) {
        if (process.env.DEBUG) {
          // eslint-disable-next-line no-console
          console.warn(`Can't get disk usage for '${result.dockerSystemInfo.DockerRootDir}': ${e}`);
        }
      }
    }

    if (!result.diskSpace) {
      try {
        result.diskSpace = await diskusage.check(os.platform() === 'win32' ? 'c:' : '/');
      } catch (e) {
        if (process.env.DEBUG) {
          // eslint-disable-next-line no-console
          console.warn(`Can't get disk usage for root directory: ${e}`);
        }
      }
    }

    return result;
  }
  return getOperatingSystemInfo;
}
