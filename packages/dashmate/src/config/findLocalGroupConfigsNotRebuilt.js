import { PRESET_LOCAL } from '../constants.js';

// The configs `setup local` builds
export const LOCAL_GROUP_MEMBER_NAME = /^local_(seed|[1-9][0-9]*)$/;

// The fewest masternodes `setup local` builds, and what it offers
const MIN_NODE_COUNT = 3;

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

/**
 * The command that builds the local group again with every masternode it has
 *
 * `setup local` offers three masternodes, so a group of more needs its count:
 * the highest local_<n>, the given names and the group's included.
 *
 * @param {ConfigFile} configFile
 * @param {string[]} [names] - members that may not be in the config file
 * @return {string}
 */
export function renderLocalGroupSetupCommand(configFile, names = []) {
  const members = configFile.isGroupExists(PRESET_LOCAL)
    ? configFile.getGroupConfigs(PRESET_LOCAL).map((config) => config.getName())
    : [];

  const nodeCount = [...names, ...members]
    .map((name) => LOCAL_GROUP_MEMBER_NAME.exec(name)?.[1])
    .filter((index) => index !== undefined && index !== 'seed')
    .reduce((count, index) => Math.max(count, Number(index)), MIN_NODE_COUNT);

  return `dashmate setup local --node-count=${nodeCount}`;
}
