/**
 * @param {Object} systemConfigs
 * @return {isSystemConfig}
 */
function isSystemConfigFactory(systemConfigs) {
  /**
   * @typedef {isSystemConfig}
   * @param configName
   * @return {boolean}
   */
  function isSystemConfig(configName) {
    const systemConfigNames = Object.keys(systemConfigs);
    return systemConfigNames.includes(configName);
  }

  return isSystemConfig;
}

module.exports = isSystemConfigFactory;
