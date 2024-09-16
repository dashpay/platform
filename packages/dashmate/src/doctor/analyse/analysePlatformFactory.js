import { SEVERITY } from '../Prescription.js';
import Problem from '../Problem.js';

/**
 * @return {analysePlatform}
 */
export default function analysePlatformFactory() {
  /**
   * @typedef {Function} analysePlatform
   * @param {Samples} samples
   * @return {Problem[]}
   */
  function analysePlatform(samples) {
    const problems = [];

    // Tenderdash is synced
    if (samples?.getDashmateConfig()?.get('platform.enable')) {
      const status = samples.getServiceInfo('drive_tenderdash', 'status');

      if (status?.sync_info?.catching_up) {
        const problem = new Problem(
          'Drive is syncing blockchain data. Some node services may temporarily be unresponsive.',
          'Please wait until Drive is fully synced',
          SEVERITY.MEDIUM,
        );

        problems.push(problem);
      }
    }

    return problems;
  }

  return analysePlatform;
}
