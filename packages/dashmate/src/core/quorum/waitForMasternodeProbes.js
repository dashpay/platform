const { LLMQ_TYPE_TEST } = require('../../constants');

/**
 * Checks all mastrenodoes probes to incterconnected masternodes
 *
 * @param {RpcClient[]} rpcClients
 * @param {Function} bumpMockTime
 *
 * @return {Promise<boolean>}
 */
async function checkProbes(rpcClients, bumpMockTime) {
  let masternodes = await Promise.all(
    rpcClients.map((rpc) => {
      const promise = rpc.masternode('status');

      return promise.then(({ result }) => ({ rpc, status: result }));
    }),
  );

  masternodes = masternodes.filter((entry) => !entry.status);

  for (const { rpc, status } of masternodes) {
    const { result: { session, quorumConnections } } = await rpc.quorum('dkgstatus');

    if (Object.keys(session).length === 0) {
      continue;
    }

    if (!quorumConnections || !quorumConnections[LLMQ_TYPE_TEST]) {
      await bumpMockTime();

      return false;
    }

    for (const connection of quorumConnections[LLMQ_TYPE_TEST]) {
      if (connection.proTxHash === status.proTxHash) {
        continue;
      }

      if (!connection.outbound) {
        const { result: mnInfo } = await rpc.protx('info', connection.proTxHash);

        for (const masternode in masternodes) {
          if (connection.proTxHash === masternode.status.proTxHash) {
            // MN is expected to be online and functioning, so let's verify that the last successful
            // probe is not too old. Probes are retried after 50 minutes, while DKGs consider
            // a probe as failed after 60 minutes
            if (mnInfo.metaInfo.lastOutboundSuccessElapsed > 55 * 60) {
              await bumpMockTime();

              return false;
            }
          // MN is expected to be offline, so let's only check that
          // the last probe is not too long ago
          } else if (mnInfo.metaInfo.lastOutboundAttemptElapsed > 55 * 60
            && mnInfo.metaInfo.lastOutboundSuccessElapsed > 55 * 60) {
            await bumpMockTime();

            return false;
          }
        }
      }
    }
  }

  return true;
}

/**
 *
 * @param {RpcClient[]} rpcClients
 * @param {Function} bumpMockTime
 * @param {number} [timeout]
 * @return {Promise<void>}
 */
async function waitForMasternodeProbes(rpcClients, bumpMockTime, timeout = 30000) {
  const deadline = Date.now() + timeout;

  let isReady = false;

  while (!isReady) {
    isReady = await checkProbes(rpcClients, bumpMockTime);

    if (Date.now() > deadline) {
      throw new Error(`waitForMasternodeProbes deadline of ${timeout} exceeded`);
    }
  }
}

module.exports = waitForMasternodeProbes;
