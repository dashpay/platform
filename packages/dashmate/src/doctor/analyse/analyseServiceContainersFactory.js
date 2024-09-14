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

    for (const service of services) {
      const dockerInspect = samples.getServiceInfo(service.name, 'dockerInspect');

      if (dockerInspect.message) {
        servicesNotStarted.push({
          service,
          message: dockerInspect.message,
        });
      } else if (
        dockerInspect.State.Restarting === true
        && dockerInspect.State.ExitCode !== 0
      ) {
        servicesFailed.push({
          service,
          message: dockerInspect.State.Error,
          code: dockerInspect.State.ExitCode,
        });
      } else if (dockerInspect.State.OOMKilled === true) {
        servicesOOMKilled.push({
          service,
        });
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
        'Please check corresponding logs or share them with Dash Core Group',
        SEVERITY.HIGH,
      );

      problems.push(problem);
    }

    if (servicesOOMKilled.length > 0) {
      let description;
      if (servicesNotStarted.length === 1) {
        description = chalk`Service ${servicesNotStarted[0].service.title} is killed due to lack of memory.`;
      } else {
        description = chalk`Services ${servicesNotStarted.map((e) => e.service.title).join(', ')} are killed due to lack of memory.`;
      }

      const problem = new Problem(
        description,
        'Make sure you have enough memory to run the node.',
        SEVERITY.HIGH,
      );

      problems.push(problem);
    }

    return problems;
  }

  return analyseServiceContainers;
}
