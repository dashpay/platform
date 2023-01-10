const preSyncedStatus = {
  test: 'test',
  sentinel_status: 'dashd not synced with network! Awaiting full sync before running Sentinel.',
  sync_asset: 'MASTERNODE_SYNC_BLOCKCHAIN',
  masternode_status: 'Waiting for ProTx to appear on-chain',
  platform_status: 'Waiting for core sync'
}

module.exports = {
  STATUS: preSyncedStatus
}
