module.exports = async function getStatus() {
  const { height, relayFee, network } = this;

  return {
    version: {
      protocol: 70218,
      software: 170000,
      agent: '/Dash Core:0.17.0/',
    },
    time: {
      now: 1616495891,
      offset: 0,
      median: 1615546573,
    },
    status: 'READY',
    syncProgress: 0.9999993798366165,
    chain: {
      name: network,
      headersCount: height,
      blocksCount: height,
      bestBlockHash: '0000007464fd8cae97830d794bf03efbeaa4b8c3258a3def67a89cdbd060f827',
      difficulty: 0.002261509525429119,
      chainWork: '000000000000000000000000000000000000000000000000022f149b98e063dc',
      isSynced: true,
      syncProgress: 0.9999993798366165,
    },
    masternode: {
      status: 'READY',
      proTxHash: '04d06d16b3eca2f104ef9749d0c1c17d183eb1b4fe3a16808fd70464f03bcd63',
      posePenalty: 0,
      isSynced: true,
      syncProgress: 1,
    },
    network: {
      peersCount: 8,
      fee: {
        relay: relayFee,
        incremental: 0.00001,
      },
    },
  };
};
