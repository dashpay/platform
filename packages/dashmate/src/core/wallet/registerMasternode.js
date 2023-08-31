/**
 * Get balance of the address
 *
 * @typedef {registerMasternode}
 * @param {CoreService} coreService
 * @param {string} collateralHash
 * @param {string} ownerAddress
 * @param {string} operatorPublicKey
 * @param {string} fundSourceAddress
 * @param {string} operatorReward
 * @param {Config} config
 * @param {boolean} [hp=false]
 * @return {Promise<string>}
 */
async function registerMasternode(
  coreService,
  collateralHash,
  ownerAddress,
  operatorPublicKey,
  fundSourceAddress,
  operatorReward,
  config,
  hp = false,
) {
  // get collateral index
  const { result: masternodeOutputs } = await coreService.getRpcClient().masternode('outputs', { wallet: 'main' });

  const collateralOutputIndex = masternodeOutputs
    .find((outpoint) => outpoint.startsWith(collateralHash))
    .split('-')[1];

  const ipAndPort = `${config.get('externalIp', true)}:${config.get('core.p2p.port')}`;

  const proTxArgs = [
    hp ? 'register_evo' : 'register',
    collateralHash, // The txid of the 1000 Dash collateral funding transaction
    parseInt(collateralOutputIndex, 10), // The output index of the 1000 Dash funding transaction
    ipAndPort, // Masternode IP address and port, in the format x.x.x.x:yyyy
    ownerAddress, // The new Dash address for the owner/voting address
    operatorPublicKey, // The Operator BLS public key
    ownerAddress, // The new Dash address, or the address of a delegate, used for proposal voting
    operatorReward, // The percentage of the block reward allocated to the operator as payment
    fundSourceAddress, // A new or existing Dash address to receive the owner’s masternode rewards
  ];

  if (hp) {
    const platformNodeId = config.get('platform.drive.tenderdash.node.id');
    const platformP2PPort = config.get('platform.drive.tenderdash.p2p.port');
    const platformHttpPort = config.get('platform.dapi.envoy.http.port');

    proTxArgs.push(platformNodeId);
    proTxArgs.push(platformP2PPort.toString());
    proTxArgs.push(platformHttpPort.toString());
  }

  const { result: proRegTxId } = await coreService.getRpcClient().protx(
    ...proTxArgs,
    { wallet: 'main' },
  );

  return proRegTxId;
}

module.exports = registerMasternode;
