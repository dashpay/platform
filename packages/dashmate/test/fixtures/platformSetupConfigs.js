import { Listr } from 'listr2';
import DefaultConfigs from '../../src/config/DefaultConfigs.js';
import ConfigFile from '../../src/config/configFile/ConfigFile.js';
import getBaseConfigFactory from '../../configs/defaults/getBaseConfigFactory.js';
import getTestnetConfigFactory from '../../configs/defaults/getTestnetConfigFactory.js';
import getLocalConfigFactory from '../../configs/defaults/getLocalConfigFactory.js';
import getMainnetConfigFactory from '../../configs/defaults/getMainnetConfigFactory.js';
import resetNodeTaskFactory from '../../src/listr/tasks/resetNodeTaskFactory.js';
import generateTenderdashNodeKey from '../../src/tenderdash/generateTenderdashNodeKey.js';
import deriveTenderdashNodeId from '../../src/tenderdash/deriveTenderdashNodeId.js';
import generateRandomString from '../../src/util/generateRandomString.js';

// A throwaway regtest operator key (see test/fixtures/coreRpc/24.0.3).
export const OPERATOR_PRIVATE_KEY = '1bbac8e471b80050ea44afb514c87cb1ef03db5c66e643c7d6e0f09b29bedbc0';

/**
 * Configs as dashmate builds them, set up the way `dashmate setup` leaves them
 * and reset the way `dashmate reset --platform --hard` leaves them
 *
 * The reset is the real reset task, so a state test cannot drift from what a
 * reset actually leaves behind.
 *
 * @param {HomeDir} homeDir
 */
export default function createPlatformSetupConfigs(homeDir) {
  const getBaseConfig = getBaseConfigFactory(homeDir);
  const getTestnetConfig = getTestnetConfigFactory(homeDir, getBaseConfig);
  const getLocalConfig = getLocalConfigFactory(getBaseConfig);
  const getMainnetConfig = getMainnetConfigFactory(homeDir, getBaseConfig);

  const defaultConfigs = new DefaultConfigs([
    getBaseConfig, getLocalConfig, getTestnetConfig, getMainnetConfig,
  ]);

  /**
   * A preset with the answers full setup gives
   *
   * @param {string} preset
   * @param {Object} options
   * @param {boolean} options.masternode
   * @param {boolean} [options.evo=true]
   * @return {Config}
   */
  function setUp(preset, { masternode, evo = true }) {
    const config = defaultConfigs.get(preset);

    const rpcUsers = structuredClone(config.get('core.rpc.users'));
    Object.values(rpcUsers).forEach((user) => {
      // eslint-disable-next-line no-param-reassign
      user.password = generateRandomString(12);
    });
    config.set('core.rpc.users', rpcUsers);

    config.set('core.masternode.enable', masternode);
    config.set('externalIp', '1.2.3.4');

    if (masternode) {
      config.set('core.masternode.operator.privateKey', OPERATOR_PRIVATE_KEY);
    }

    config.set('platform.enable', evo);
    config.set('platform.drive.tenderdash.mode', masternode ? 'validator' : 'full');

    if (evo) {
      const key = generateTenderdashNodeKey();
      config.set('platform.drive.tenderdash.node.key', key);
      config.set('platform.drive.tenderdash.node.id', deriveTenderdashNodeId(key));
      config.set('platform.gateway.ssl.enabled', true);
      config.set('platform.gateway.ssl.provider', 'letsencrypt');
    }

    return config;
  }

  /**
   * @param {Config} config
   * @param {Object} [options]
   * @param {boolean} [options.keepData=false]
   * @return {Promise<Config>} the same config, reset
   */
  async function hardResetPlatform(config, { keepData = false } = {}) {
    const configFile = new ConfigFile([getBaseConfig(), config], '5.2.0', 'abcdef12', config.getName(), null);

    const resetNodeTask = resetNodeTaskFactory(
      {
        isNodeRunning: async () => false,
        rm: async () => {},
        getVolumeNames: async () => [],
      },
      {},
      null,
      null,
      defaultConfigs,
      configFile,
      homeDir,
      () => [{ name: 'drive_abci', profiles: ['platform'] }],
    );

    await new Listr([{ task: () => resetNodeTask(config) }], { renderer: 'silent' }).run({
      isHardReset: true,
      isPlatformOnlyReset: true,
      isForce: true,
      keepData,
      removeConfig: false,
    });

    return config;
  }

  return {
    defaultConfigs,
    setUp,
    hardResetPlatform,
  };
}
