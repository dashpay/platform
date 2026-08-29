/**
 * Wire a node config into a local network's Tenderdash mesh: shared chain id,
 * persistent peers (all given peer configs except the node itself) and the
 * validator quorum type used in the genesis document.
 *
 * @param {Config} config - config of the node to wire
 * @param {string} chainId - chain id shared by the local network
 * @param {Config[]} peerConfigs - platform-enabled configs of the network
 * @return {void}
 */
export default function wireLocalTenderdashNode(config, chainId, peerConfigs) {
  config.set('platform.drive.tenderdash.genesis.chain_id', chainId);

  const p2pPeers = peerConfigs
    .filter((peerConfig) => peerConfig.getName() !== config.getName())
    .map((peerConfig) => ({
      id: peerConfig.get('platform.drive.tenderdash.node.id'),
      host: peerConfig.get('externalIp'),
      port: peerConfig.get('platform.drive.tenderdash.p2p.port'),
    }));

  config.set('platform.drive.tenderdash.p2p.persistentPeers', p2pPeers);

  config.set(
    'platform.drive.tenderdash.genesis.validator_quorum_type',
    config.get('platform.drive.abci.validatorSet.quorum.llmqType'),
  );
}
