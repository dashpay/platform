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

      // Config option 'platform.dapi.deprecated.enabled' was removed
      const deprecatedEnabled = config.has('platform.dapi.deprecated.enabled')
        ? config.get('platform.dapi.deprecated.enabled')
        : false;

      if (includeAll) {
        profiles.push('platform-dapi-deprecated');
        profiles.push('platform-dapi-rs');
      } else if (deprecatedEnabled) {
        profiles.push('platform-dapi-deprecated');
      } else {
        profiles.push('platform-dapi-rs');
      }
    }

    if (config.get('platform.quorumList.enabled')) {
      profiles.push('platform-quorum');
    }

    return Array.from(new Set(profiles));
  }

  return getConfigProfiles;
}
