/**
 * @return {getConfigProfiles}
 */
export default function getConfigProfilesFactory() {
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
