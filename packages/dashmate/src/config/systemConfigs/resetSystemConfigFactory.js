/**
 * @param {Object} systemConfigs
 * @return {resetSystemConfig}
 */
function resetSystemConfigFactory(systemConfigs) {
  /**
   * @typedef {resetSystemConfig}
   *
   * @param {ConfigCollection} configCollection
   * @param {string} name
   */
  function resetSystemConfig(configCollection, name) {
    const systemConfigNames = Object.keys(systemConfigs);
    if (!systemConfigNames.includes(name)) {
      throw new Error(`Only system configs can be reset: ${systemConfigNames.join(', ')}`);
    }

    configCollection.getConfig(name).setOptions(systemConfigs[name]);
  }

  return resetSystemConfig;
}

module.exports = resetSystemConfigFactory;
