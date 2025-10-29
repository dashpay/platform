/**
 * @return {getConfigProfiles}
 */
export default function getConfigProfilesFactory() {
  /**
   * @typedef {function} getConfigProfiles
   * @param {Config} config
   * @param {{ includeAll?: boolean }} [options]
   * @returns {string[]}
   */
  function getConfigProfiles(config, { includeAll = false } = {}) {
    const profiles = [];

    profiles.push('core');

    if (config.get('platform.enable')) {
      profiles.push('platform');

      const deprecatedEnabled = config.get('platform.dapi.deprecated.enabled');

      if (includeAll) {
        profiles.push('platform-dapi-deprecated');
        profiles.push('platform-dapi-rs');
      } else if (deprecatedEnabled) {
        profiles.push('platform-dapi-deprecated');
      } else {
        profiles.push('platform-dapi-rs');
      }
    }

    return Array.from(new Set(profiles));
  }

  return getConfigProfiles;
}
