const preSyncedStatus = {
  testChain: 'test',
  testnet: 'testnet',
  sentinel_statusNotSynced: 'dashd not synced with network! Awaiting full sync before running Sentinel.',
  masternode_status: 'Waiting for ProTx to appear on-chain',
  platform_status: 'Waiting for core sync',
};

module.exports = {
  STATUS: preSyncedStatus,
};
