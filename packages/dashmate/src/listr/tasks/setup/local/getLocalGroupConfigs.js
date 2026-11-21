import { PRESET_LOCAL } from '../../../../constants.js';

/**
 * Get the configs of a local network's nodes, creating the missing ones from the local preset
 *
 * A node added to an existing network runs the Dash Core image its peers run. Core v24
 * marks older members bad in DKG, and does not ask v23 peers for sporks, so a newer
 * node beside them runs without quorums or ChainLocks.
 *
 * @param {ConfigFile} configFile
 * @param {string[]} configNames
 * @return {Config[]}
 */
export default function getLocalGroupConfigs(configFile, configNames) {
  const existingConfigs = configNames
    .filter((configName) => configFile.isConfigExists(configName))
    .map((configName) => configFile.getConfig(configName));

  const coreImage = existingConfigs[0]?.get('core.docker.image');

  return configNames.map((configName) => {
    if (configFile.isConfigExists(configName)) {
      return configFile.getConfig(configName);
    }

    const config = configFile.createConfig(configName, PRESET_LOCAL);

    if (coreImage !== undefined) {
      config.set('core.docker.image', coreImage);
    }

    return config;
  });
}
