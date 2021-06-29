const wait = require('../../util/wait');

const { LLMQ_TYPE_TEST } = require('../../constants');

/**
 * @param {RpcClient} rpcClient
 * @param {number} expectedConnectionsCount
 * @return {Promise<boolean>}
 */
async function checkQuorumConnections(rpcClient, expectedConnectionsCount) {
  const { result: dkgStatus } = await rpcClient.quorum('dkgstatus');

  if (Object.keys(dkgStatus.session).length === 0) {
    return false;
  }

  const noConnections = dkgStatus.quorumConnections == null;
  const llmqConnections = dkgStatus.quorumConnections;

  if (noConnections || llmqConnections[LLMQ_TYPE_TEST] == null) {
    return false;
  }

  const connectionsCount = llmqConnections[LLMQ_TYPE_TEST]
    .filter((connection) => connection.connected)
    .length;

  return connectionsCount >= expectedConnectionsCount;
}

/**
 *
 * @param {RpcClient[]} rpcClients
 * @param {number} expectedConnectionsCount
 * @param {Function} bumpMockTime
 * @param {number} [timeout]
 * @return {Promise<void>}
 */
async function waitForQuorumConnections(
  rpcClients,
  expectedConnectionsCount,
  bumpMockTime,
  timeout = 300000,
) {
  const deadline = Date.now() + timeout;
  const readyNodes = new Set();
  const nodesToWait = 3;

  while (readyNodes.size < nodesToWait) {
    await Promise.all(rpcClients.map(async (rpcClient, i) => {
      const isReady = await checkQuorumConnections(
        rpcClient,
        expectedConnectionsCount,
      );

      if (isReady) {
        readyNodes.add(i);
      }
    }));

    if (readyNodes.size < nodesToWait) {
      await bumpMockTime();

      await wait(1000);
    }

    if (Date.now() > deadline) {
      throw new Error(`waitForQuorumConnections deadline of ${timeout} exceeded`);
    }
  }
}

module.exports = waitForQuorumConnections;
