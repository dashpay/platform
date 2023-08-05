/**
 * @return {getConfigProfiles}
 */
function getConfigProfilesFactory() {
  function getConfigProfiles(config) {
    const profiles = [];

    profiles.push('core');

    if (config.get('core.masternode.enable')) {
      profiles.push('masternode');
    }

    if (config.get('platform.enable')) {
      profiles.push('platform');
    }

    return profiles;
  }

  return getConfigProfiles;
}

module.exports = getConfigProfilesFactory;
