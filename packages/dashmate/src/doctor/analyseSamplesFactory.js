import { Prescription, SEVERITY } from './Prescription.js';
import Problem from './Problem.js';

/**
 * @param {analyseSystemResources} analyseSystemResources
 * @param {analyseServiceContainers} analyseServiceContainers
 * @param {analyseConfig} analyseConfig
 * @param {analyseCore} analyseCore
 * @param {analysePlatform} analysePlatform
 * @return {analyseSamples}
 */
export default function analyseSamplesFactory(
  analyseSystemResources,
  analyseServiceContainers,
  analyseConfig,
  analyseCore,
  analysePlatform,
) {
  /**
   * @typedef {Function} analyseSamples
   * @param {Samples} samples
   * @return {Prescription}
   */
  function analyseSamples(samples) {
    const problems = [];

    // System resources
    problems.push(...analyseSystemResources(samples));

    // Docker
    const dockerError = samples.getStringifiedDockerError();
    if (dockerError) {
      problems.push(new Problem(
        'Docker installation error',
        dockerError,
        SEVERITY.HIGH,
      ));
    }

    problems.push(...analyseServiceContainers(samples));

    problems.push(...analyseConfig(samples));

    problems.push(...analyseCore(samples));

    problems.push(...analysePlatform(samples));

    return new Prescription(problems);
  }

  return analyseSamples;
}
