/**
 * @param {Object} systemConfigs
 * @return {checkSystemConfig}
 */
function checkSystemConfigFactory(systemConfigs) {
  /**
   * @typedef {checkSystemConfig}
   * @param configName
   * @return {boolean}
   */
  function checkSystemConfig(configName) {
    const systemConfigNames = Object.keys(systemConfigs);
    return systemConfigNames.includes(configName);
  }

  return checkSystemConfig;
}

module.exports = checkSystemConfigFactory;
