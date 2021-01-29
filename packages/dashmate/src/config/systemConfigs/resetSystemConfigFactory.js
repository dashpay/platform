/**
 * @param {Object} systemConfigs
 * @return {resetSystemConfig}
 */
function resetSystemConfigFactory(systemConfigs) {
  /**
   * @typedef {resetSystemConfig}
   * @param {ConfigCollection} configCollection
   * @param {string} name
   * @param {boolean} platformOnly
   */
  function resetSystemConfig(configCollection, name, platformOnly = false) {
    if (platformOnly) {
      const { platform: systemPlatformConfig } = systemConfigs[name];
      configCollection.getConfig(name).set('platform', systemPlatformConfig);
    } else {
      configCollection.getConfig(name).setOptions(systemConfigs[name]);
    }
  }

  return resetSystemConfig;
}

module.exports = resetSystemConfigFactory;
