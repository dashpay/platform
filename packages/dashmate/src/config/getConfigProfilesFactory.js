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
  function getConfigProfiles(config) {
    const profiles = [];

    profiles.push('core');

    if (config.get('platform.enable')) {
      profiles.push('platform');
    }

    return Array.from(new Set(profiles));
  }

  return getConfigProfiles;
}
