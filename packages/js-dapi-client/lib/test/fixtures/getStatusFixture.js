/**
 *
 */
function getStatusFixture() {
  return {
    version: {
      software: {
        dapi: '1.8.0-rc.2',
        drive: '1.8.0-rc.3',
        tenderdash: '1.4.0',
      },
      protocol: {
        tenderdash: {
          p2p: 10,
          block: 14,
        },
        drive: {
          latest: 9,
          current: 8,
          nextEpoch: 10,
        },
      },
    },
    node: {
      id: new Uint8Array(Buffer.from('QbMI9zfKnjn2e1UxWJAxmKiMUW4=', 'base64')),
      proTxHash: new Uint8Array(Buffer.from('s7V0hXG2D+mtEScV1qUXJdblpSqcOvX9NqFyTPUNhi8=', 'base64')),
    },
    chain: {
      catchingUp: false,
      latestBlockHash: new Uint8Array(Buffer.from('mVDwGtY2oJSaLLgv3WpLp2dFDyFEtqhD4z1gl2OJceY=', 'base64')),
      latestAppHash: new Uint8Array(Buffer.from('jHgEBK8aZ74TUKcUGN58EFzUNvNsLboOgYe6eH/JetU=', 'base64')),
      latestBlockHeight: '94461',
      earliestBlockHash: new Uint8Array(Buffer.from('CPoCwn7AOQujAeT8fj1+rbNQyBk+PmKgk2iXBuOiC/o=', 'base64')),
      earliestAppHash: new Uint8Array(Buffer.from('vwzLnKBxugGubmegwJD5eAPSbVbWddzVExeBy8rI7I8=', 'base64')),
      earliestBlockHeight: '1',
      maxPeerBlockHeight: '94461',
      coreChainLockedHeight: 1187358,
    },
    network: {
      chainId: 'dash-testnet-51',
      peersCount: 96,
      listening: true,
    },
    stateSync: {
      totalSyncedTime: '2312323',
      remainingTime: '1337',
      totalSnapshots: 300,
      chunkProcessAverageTime: '213123',
      snapshotHeight: '10000',
      snapshotChunksCount: '1000',
      backfilledBlocks: '1400',
      backfillBlocksTotal: '2000',
    },
    time: {
      local: '1738336806994',
      block: '1738336736273',
      genesis: '0',
      epoch: 4717,
    },
  };
}

module.exports = getStatusFixture;
