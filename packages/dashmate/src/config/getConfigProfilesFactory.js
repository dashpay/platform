/**
 * @return {getConfigProfiles}
 */
export default function getConfigProfilesFactory() {
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

      // Select which DAPI stack to enable via profiles
      if (config.get('platform.dapi.deprecated.enabled')) {
        profiles.push('platform-dapi-deprecated');
      } else {
        profiles.push('platform-dapi-rs');
      }
    }

    return profiles;
  }

  return getConfigProfiles;
}
