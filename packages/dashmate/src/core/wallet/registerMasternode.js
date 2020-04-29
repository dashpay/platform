/**
 * Get balance of the address
 *
 * @typedef {registerMasternode}
 * @param {CoreService} coreService
 * @param {string} collateralHash
 * @param {string} masternodeExternalIp
 * @param {number} masternodeP2PPort
 * @param {string} ownerAddress
 * @param {string} operatorPublicKey
 * @param {string} fundSourceAddress
 * @return {Promise<string>}
 */
async function registerMasternode(
  coreService,
  collateralHash,
  masternodeExternalIp,
  masternodeP2PPort,
  ownerAddress,
  operatorPublicKey,
  fundSourceAddress,
) {
  // get collateral index
  const { result: masternodeOutputs } = await coreService.getRpcClient().masternode('outputs');

  const collateralIndex = parseInt(masternodeOutputs[collateralHash], 10);

  const { result: proRegTxId } = await coreService.getRpcClient().protx(
    'register',
    collateralHash, // The txid of the 1000 Dash collateral funding transaction
    collateralIndex, // The output index of the 1000 Dash funding transaction
    `${masternodeExternalIp}:${masternodeP2PPort}`, // Masternode IP address and port, in the format x.x.x.x:yyyy
    ownerAddress, // The new Dash address for the owner/voting address
    operatorPublicKey, // The Operator BLS public key
    ownerAddress, // The new Dash address, or the address of a delegate, used for proposal voting
    0, // The percentage of the block reward allocated to the operator as payment
    fundSourceAddress, // A new or existing Dash address to receive the owner’s masternode rewards
  );

  return proRegTxId;
}

module.exports = registerMasternode;
