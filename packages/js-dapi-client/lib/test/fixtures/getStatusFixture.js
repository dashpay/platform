import { base64ToBytes } from '../../utils/bytes.js';

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
      id: base64ToBytes('QbMI9zfKnjn2e1UxWJAxmKiMUW4='),
      proTxHash: base64ToBytes('s7V0hXG2D+mtEScV1qUXJdblpSqcOvX9NqFyTPUNhi8='),
    },
    chain: {
      catchingUp: false,
      latestBlockHash: base64ToBytes('mVDwGtY2oJSaLLgv3WpLp2dFDyFEtqhD4z1gl2OJceY='),
      latestAppHash: base64ToBytes('jHgEBK8aZ74TUKcUGN58EFzUNvNsLboOgYe6eH/JetU='),
      latestBlockHeight: '94461',
      earliestBlockHash: base64ToBytes('CPoCwn7AOQujAeT8fj1+rbNQyBk+PmKgk2iXBuOiC/o='),
      earliestAppHash: base64ToBytes('vwzLnKBxugGubmegwJD5eAPSbVbWddzVExeBy8rI7I8='),
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

export default getStatusFixture;
