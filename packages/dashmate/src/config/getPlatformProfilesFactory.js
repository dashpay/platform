const PLATFORM_PROFILES = [
  'platform',
  'platform-dapi-deprecated',
  'platform-dapi-rs',
];

/**
 * @param {getConfigProfiles} getConfigProfiles
 * @return {getPlatformProfiles}
 */
export default function getPlatformProfilesFactory(getConfigProfiles) {
  /**
   * @typedef {function} getPlatformProfiles
   * @param {Config} config
   * @param {{includeAll?: boolean}} [options]
   * @returns {string[]}
   */
  function getPlatformProfiles(config, { includeAll = false } = {}) {
    if (!config.get('platform.enable')) {
      return [];
    }

    if (includeAll) {
      return [...PLATFORM_PROFILES];
    }

    return getConfigProfiles(config)
      .filter((profile) => profile.startsWith('platform'));
  }

  return getPlatformProfiles;
}
