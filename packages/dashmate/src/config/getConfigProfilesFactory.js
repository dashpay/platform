/**
 * @return {getConfigProfiles}
 */
function getConfigProfilesFactory() {
  /**
   * @typedef {function} getConfigProfiles
   * @param {Config} config
   * @returns {string[]}
   */
  function getConfigProfiles(config) {
    const profiles = [];

    profiles.push('core');

    if (config.get('platform.enable')) {
      profiles.push('platform');
    }

    return profiles;
  }

  return getConfigProfiles;
}

module.exports = getConfigProfilesFactory;
