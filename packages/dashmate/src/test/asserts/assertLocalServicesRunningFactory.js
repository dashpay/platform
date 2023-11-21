import SERVICES from '../constants/services.js';

/**
 * @param {assertServiceRunning} assertServiceRunning
 *
 * @returns {assertLocalServicesRunning}
 */
export default function assertLocalServicesRunningFactory(assertServiceRunning) {
  /**
   * Check all node services are up and running
   *
   * @typedef {assertLocalServicesRunning}
   * @param {Array<Config>} configGroup
   * @param {boolean} [expected=false]
   */
  async function assertLocalServicesRunning(configGroup, expected = true) {
    for (const config of configGroup) {
      if (config.name === 'local_seed') {
        await assertServiceRunning(config, 'core', expected);
      } else {
        for (const serviceName of Object.keys(SERVICES)) {
          await assertServiceRunning(config, serviceName, expected);
        }
      }
    }
  }

  return assertLocalServicesRunning;
}
