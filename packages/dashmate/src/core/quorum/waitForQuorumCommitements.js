const wait = require('../../util/wait');

const { LLMQ_TYPE_TEST } = require('../../constants');

/**
 *
 * @param {string} quorumHash
 * @param {RpcClient[]} rpcClients
 * @return {Promise<boolean>}
 */
async function checkDKGSessionCommitments(quorumHash, rpcClients) {
  for (const rpc of rpcClients) {
    const { result: dkgStatus } = await rpc.quorum('dkgstatus');

    if (!dkgStatus.minableCommitments) {
      return false;
    }

    const testQuorumCommitment = dkgStatus.minableCommitments[LLMQ_TYPE_TEST];

    if (!testQuorumCommitment) {
      return false;
    }

    if (testQuorumCommitment.quorumHash !== quorumHash) {
      return false;
    }
  }

  return true;
}

/**
 *
 * @param {RpcClient[]} rpcClients
 * @param {string} quorumHash
 * @param {number} [timeout]
 * @param {number} [waitBeforeRetry]
 * @return {Promise<void>}
 */
async function waitForQuorumCommitments(
  rpcClients,
  quorumHash,
  timeout = 60000,
  waitBeforeRetry = 100,
) {
  const deadline = Date.now() + timeout;
  let isReady = false;

  while (!isReady) {
    await wait(waitBeforeRetry);

    isReady = await checkDKGSessionCommitments(quorumHash, rpcClients);

    if (Date.now() > deadline) {
      throw new Error(`waitForQuorumCommitments deadline of ${timeout} exceeded`);
    }
  }
}

module.exports = waitForQuorumCommitments;
