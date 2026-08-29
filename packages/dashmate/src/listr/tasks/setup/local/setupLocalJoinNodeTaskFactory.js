import { Listr } from 'listr2';
import lodash from 'lodash';
import {
  PRESET_LOCAL,
} from '../../../../constants.js';
import deriveTenderdashNodeId from '../../../../tenderdash/deriveTenderdashNodeId.js';
import generateTenderdashNodeKey from '../../../../tenderdash/generateTenderdashNodeKey.js';
import generateRandomString from '../../../../util/generateRandomString.js';
import wireLocalTenderdashNode from './wireLocalTenderdashNode.js';

const { cloneDeep: lodashCloneDeep } = lodash;

/**
 * Config option paths that get a per-node host port offset on local networks.
 * Mirrors the offsets applied to validators in setupLocalPresetTaskFactory.
 *
 * @type {string[]}
 */
const OFFSET_PORT_OPTIONS = [
  'core.p2p.port',
  'core.rpc.port',
  'core.zmq.port',
  'dashmate.helper.api.port',
  'platform.drive.abci.grovedbVisualizer.port',
  'platform.drive.abci.tokioConsole.port',
  'platform.drive.abci.metrics.port',
  'platform.dapi.rsDapi.metrics.port',
  'platform.gateway.admin.port',
  'platform.gateway.listeners.dapiAndDrive.port',
  'platform.gateway.metrics.port',
  'platform.gateway.rateLimiter.metrics.port',
  'platform.drive.tenderdash.p2p.port',
  'platform.drive.tenderdash.rpc.port',
  'platform.drive.tenderdash.pprof.port',
  'platform.drive.tenderdash.metrics.port',
];

/**
 * @param {ConfigFile} configFile
 * @param {resolveDockerHostIp} resolveDockerHostIp
 * @param {obtainSelfSignedCertificateTask} obtainSelfSignedCertificateTask
 * @return {setupLocalJoinNodeTask}
 */
export default function setupLocalJoinNodeTaskFactory(
  configFile,
  resolveDockerHostIp,
  obtainSelfSignedCertificateTask,
) {
  /**
   * Create a `local_join` config (or `configName`) for a full node (not a
   * masternode, so no collateral registration) that joins an already set up
   * local network and bootstraps Drive from a state sync snapshot. Kept as a
   * DI task rather than a `group join` CLI command while the state sync e2e
   * test is its only consumer.
   *
   * @typedef {setupLocalJoinNodeTask}
   * @param {Config[]} groupConfigs - configs of the existing local group
   * @param {Object} [options]
   * @param {string} [options.configName] - name for the new config
   * @param {number} [options.offsetIndex] - host port offset slot to occupy
   * @return {Listr}
   */
  function setupLocalJoinNodeTask(groupConfigs, options = {}) {
    const {
      configName = 'local_join',
      // Local nodes local_1..local_N occupy offset indexes 0..N-1 and
      // local_seed occupies N, so the first joining node continues at N + 1.
      // A caller adding a second joiner passes the next slot explicitly so the
      // two do not land on the same host ports.
      offsetIndex = groupConfigs.length,
    } = options;

    return new Listr([
      {
        title: 'Create join node config',
        task: async (ctx) => {
          const nodeIndex = offsetIndex + 1;

          const config = configFile.createConfig(configName, PRESET_LOCAL);

          config.set('group', 'local');
          config.set('description', 'full node joining the local network');

          OFFSET_PORT_OPTIONS.forEach((optionPath) => {
            config.set(optionPath, config.get(optionPath) + (offsetIndex * 100));
          });

          // Reads hand back a frozen snapshot, so build the new value and set
          // it back rather than writing through the object get() returned.
          const rpcUsers = lodashCloneDeep(config.get('core.rpc.users'));
          Object.values(rpcUsers).forEach((rpcUser) => {
            // eslint-disable-next-line no-param-reassign
            rpcUser.password = generateRandomString(12);
          });
          config.set('core.rpc.users', rpcUsers);

          config.set('externalIp', await resolveDockerHostIp());

          const subnet = config.get('docker.network.subnet').split('.');
          subnet[2] = nodeIndex;
          config.set('docker.network.subnet', subnet.join('.'));

          // A regular full node: no masternode registration is needed
          config.set('core.masternode.enable', false);

          // Sync Core from the existing network
          const seedConfigs = groupConfigs.filter((groupConfig) => (
            groupConfig.getName() === 'local_seed'
          ));

          if (seedConfigs.length === 0) {
            throw new Error('Cannot join the local network: no local_seed config in the group');
          }

          config.set('core.p2p.seeds', seedConfigs.map((seedConfig) => ({
            host: seedConfig.get('externalIp'),
            port: seedConfig.get('core.p2p.port'),
          })));

          // Every group config carries the same spork keys (set during setup)
          config.set('core.spork.address', groupConfigs[0].get('core.spork.address'));
          config.set('core.spork.privateKey', groupConfigs[0].get('core.spork.privateKey'));

          // Platform full node with a fresh Tenderdash identity
          config.set('platform.drive.tenderdash.mode', 'full');

          const nodeKey = generateTenderdashNodeKey();

          config.set('platform.drive.tenderdash.node.id', deriveTenderdashNodeId(nodeKey));
          config.set('platform.drive.tenderdash.node.key', nodeKey);
          config.set('platform.drive.tenderdash.moniker', config.getName());

          // A fresh node with state sync enabled bootstraps from a snapshot
          // served by the existing nodes instead of replaying blocks
          config.set('platform.drive.tenderdash.stateSync.enabled', true);

          // Join the existing Tenderdash mesh and genesis
          const platformConfigs = groupConfigs.filter((groupConfig) => (
            groupConfig.get('platform.enable')
          ));

          if (platformConfigs.length === 0) {
            throw new Error('Cannot join the local network: no platform-enabled configs in the group');
          }

          const chainId = platformConfigs[0].get('platform.drive.tenderdash.genesis.chain_id');

          wireLocalTenderdashNode(config, chainId, platformConfigs);

          ctx.joinNodeConfig = config;
        },
      },
      {
        title: 'Configure SSL certificate',
        task: (ctx) => obtainSelfSignedCertificateTask(ctx.joinNodeConfig),
      },
    ]);
  }

  return setupLocalJoinNodeTask;
}
