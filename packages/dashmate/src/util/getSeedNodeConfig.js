/**
 * @param {Config[]} configGroup
 * @return {Config}
 */
function getSeedNodeConfig(configGroup) {
  return configGroup.find((config) => config.getName() === 'local_seed');
}

module.exports = getSeedNodeConfig;
