import chalk from 'chalk';
import { SEVERITY } from '../Prescription.js';
import Problem from '../Problem.js';

/**
 * @param {getServiceList} getServiceList
 * @return {analyseServiceContainers}
 */
export default function analyseServiceContainersFactory(
  getServiceList,
) {
  /**
   * @typedef {analyseServiceContainers}
   * @param {Samples} samples
   * @return {Problem[]}
   */
  function analyseServiceContainers(samples) {
    const services = getServiceList(samples.getDashmateConfig());

    const servicesNotStarted = [];
    const servicesFailed = [];
    const servicesOOMKilled = [];
    const servicesHighCpuUsage = [];
    const servicesHighMemoryUsage = [];

    for (const service of services) {
      const dockerInspect = samples.getServiceInfo(service.name, 'dockerInspect');
      const dockerStats = samples.getServiceInfo(service.name, 'dockerStats');

      if (!dockerInspect) {
        continue;
      }

      if (dockerInspect.message) {
        servicesNotStarted.push({
          service,
          message: dockerInspect.message,
        });
      } else if (
        dockerInspect.State?.Restarting === true
        && dockerInspect.State?.ExitCode !== 0
      ) {
        servicesFailed.push({
          service,
          message: dockerInspect.State.Error,
          code: dockerInspect.State.ExitCode,
        });
      } else if (dockerInspect.State?.OOMKilled === true) {
        servicesOOMKilled.push({
          service,
        });
      }

      const cpuSystemUsage = dockerStats?.cpuStats?.system_cpu_usage ?? 0;
      const cpuServiceUsage = dockerStats?.cpuStats?.cpu_usage?.total_usage ?? 0;

      if (cpuSystemUsage > 0) {
        const cpuUsage = cpuServiceUsage / cpuSystemUsage;

        if (cpuUsage > 0.8) {
          servicesHighCpuUsage.push({
            service,
            cpuUsage,
          });
        }
      }

      const memoryLimit = dockerStats?.memoryStats?.limit ?? 0;
      const memoryServiceUsage = dockerStats?.memoryStats?.usage ?? 0;

      if (memoryLimit > 0) {
        const memoryUsage = memoryServiceUsage / memoryLimit;

        if (memoryUsage > 0.8) {
          servicesHighMemoryUsage.push({
            service,
            memoryUsage,
          });
        }
      }
    }

    const problems = [];

    if (servicesNotStarted.length > 0) {
      let description;
      if (servicesNotStarted.length === 1) {
        description = `Service ${servicesNotStarted[0].service.title} isn't started.`;
      } else {
        description = `Services ${servicesNotStarted.map((e) => e.service.title).join(', ')} aren't started.`;
      }

      const problem = new Problem(
        description,
        chalk`Try {bold.cyanBright dashmate start --force} to make sure all services are started`,
        SEVERITY.HIGH,
      );

      problems.push(problem);
    }

    for (const failedService of servicesFailed) {
      let description = `Service ${failedService.service.title} failed with an error code ${failedService.code}`;

      if (failedService.message?.length > 0) {
        description += `\nand message: ${failedService.message}`;
      }

      description += '.';

      const problem = new Problem(
        description,
        'Please check service logs or share them with Dash Core Group',
        SEVERITY.HIGH,
      );

      problems.push(problem);
    }

    if (servicesOOMKilled.length > 0) {
      let description;
      if (servicesNotStarted.length === 1) {
        description = chalk`Service ${servicesNotStarted[0].service.title} was killed due to a lack of memory.`;
      } else {
        description = chalk`Services ${servicesNotStarted.map((e) => e.service.title).join(', ')} were killed due to lack of memory.`;
      }

      const problem = new Problem(
        description,
        'Make sure you have enough memory to run the node.',
        SEVERITY.HIGH,
      );

      problems.push(problem);
    }

    if (servicesHighCpuUsage.length > 0) {
      for (const highCpuService of servicesHighCpuUsage) {
        const description = `Service ${highCpuService.service.title} is consuming ${(highCpuService.cpuUsage * 100).toFixed(2)}% CPU.`;

        const problem = new Problem(
          description,
          'Consider upgrading CPU or report in case of misbehaviour.',
          SEVERITY.MEDIUM,
        );

        problems.push(problem);
      }
    }

    if (servicesHighMemoryUsage.length > 0) {
      for (const highMemoryService of servicesHighMemoryUsage) {
        const description = `Service ${highMemoryService.service.title} is consuming ${(highMemoryService.memoryUsage * 100).toFixed(2)}% RAM.`;

        const problem = new Problem(
          description,
          'Consider upgrading RAM or report in case of misbehaviour.',
          SEVERITY.MEDIUM,
        );

        problems.push(problem);
      }
    }

    return problems;
  }

  return analyseServiceContainers;
}
