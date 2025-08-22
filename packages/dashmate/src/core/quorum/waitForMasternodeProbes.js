import { LLMQ_TYPE_TEST } from '../../constants.js';
import wait from '../../util/wait.js';

/**
 * Checks all mastrenodoes probes to incterconnected masternodes
 *
 * @param {RpcClient[]} rpcClients
 *
 * @return {Promise<boolean>}
 */
async function checkProbes(rpcClients) {
  let masternodes = await Promise.all(
    rpcClients.map((rpc) => {
      const promise = rpc.masternode('status');

      return promise.then(({ result }) => ({ rpc, status: result }));
    }),
  );

  masternodes = masternodes.filter((entry) => !entry.status);

  for (const { rpc, status } of masternodes) {
    const { result: { session, quorumConnections } } = await rpc.quorum('dkgstatus', 2);

    if (session.length === 0) {
      continue;
    }

    const llmqConnection = quorumConnections
      .find((connection) => connection.llmqType === LLMQ_TYPE_TEST);

    if (!llmqConnection) {
      return false;
    }

    for (const connection of llmqConnection.quorumConnections) {
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
              return false;
            }
          // MN is expected to be offline, so let's only check that
          // the last probe is not too long ago
          } else if (mnInfo.metaInfo.lastOutboundAttemptElapsed > 55 * 60
            && mnInfo.metaInfo.lastOutboundSuccessElapsed > 55 * 60) {
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
 * @param {number} [timeout]
 * @return {Promise<void>}
 */
export default async function waitForMasternodeProbes(rpcClients, timeout = 30000) {
  const deadline = Date.now() + timeout;

  let isReady = false;

  while (!isReady) {
    isReady = await checkProbes(rpcClients);

    if (Date.now() > deadline) {
      throw new Error(`waitForMasternodeProbes deadline of ${timeout} exceeded`);
    }

    await wait(100);
  }
}
