import { PRESET_LOCAL } from '../constants.js';

// The configs `setup local` builds
export const LOCAL_GROUP_MEMBER_NAME = /^local_(seed|[1-9][0-9]*)$/;

/**
 * The configs of the local group that `setup local` would not build again
 *
 * `group reset --group local --hard` removes every config whose group is
 * local, and a config created from one of them inherits that group, so the
 * rebuild is safe advice only while this is empty.
 *
 * @param {ConfigFile} configFile
 * @return {string[]} their names
 */
export default function findLocalGroupConfigsNotRebuilt(configFile) {
  if (!configFile.isGroupExists(PRESET_LOCAL)) {
    return [];
  }

  return configFile.getGroupConfigs(PRESET_LOCAL)
    .map((config) => config.getName())
    .filter((name) => !LOCAL_GROUP_MEMBER_NAME.test(name));
}
