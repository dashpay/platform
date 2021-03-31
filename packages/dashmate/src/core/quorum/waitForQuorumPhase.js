const wait = require('../../util/wait');

const { LLMQ_TYPE_TEST } = require('../../constants');

/**
 *
 * @param {RpcClient[]} rpcClients
 * @param {string} quorumHash
 * @param {number} phase
 * @param {number} expectedMemberCount
 * @param {string} [checkReceivedMessagesType]
 * @param {number} [checkReceivedMessagesCount]
 * @return {Promise<boolean>}
 */
async function checkDKGSessionPhase(
  rpcClients,
  quorumHash,
  phase,
  expectedMemberCount,
  checkReceivedMessagesType,
  checkReceivedMessagesCount = 0,
) {
  let memberCount = 0;

  for (const rpcClient of rpcClients) {
    const { result: dkgStatus } = await rpcClient.quorum('dkgstatus');
    const { session } = dkgStatus;

    if (!Object.prototype.hasOwnProperty.call(session, LLMQ_TYPE_TEST)) {
      continue;
    }

    memberCount += 1;

    const llmqSession = session[LLMQ_TYPE_TEST];

    const quorumHashDoesntMatch = llmqSession.quorumHash !== quorumHash;

    const sessionPhaseDoesntMatch = !Object.prototype.hasOwnProperty.call(llmqSession, 'phase')
      || llmqSession.phase !== phase;

    const receivedMessagesDoNotMatch = checkReceivedMessagesType
      && (llmqSession[checkReceivedMessagesType] < checkReceivedMessagesCount);

    const checkFailed = quorumHashDoesntMatch
      || sessionPhaseDoesntMatch
      || receivedMessagesDoNotMatch;

    if (checkFailed) {
      return false;
    }
  }

  return memberCount === expectedMemberCount;
}

/**
 *
 * @param {RpcClient[]} rpcClients
 * @param {string} quorumHash
 * @param {number} phase
 * @param {number} expectedMemberCount
 * @param {string} [checkReceivedMessagesType]
 * @param {number} [checkReceivedMessagesCount]
 * @param {number} [timeout]
 * @param {number} [checkInterval]
 * @return {Promise<void>}
 */
async function waitForQuorumPhase(
  rpcClients,
  quorumHash,
  phase,
  expectedMemberCount,
  checkReceivedMessagesType,
  checkReceivedMessagesCount,
  timeout = 30000,
  checkInterval = 100,
) {
  const deadline = Date.now() + timeout;

  let isReady = false;

  while (isReady) {
    await wait(checkInterval);

    isReady = await checkDKGSessionPhase(
      rpcClients,
      quorumHash,
      phase,
      expectedMemberCount,
      checkReceivedMessagesType,
      checkReceivedMessagesCount,
    );

    if (Date.now() > deadline) {
      throw new Error(`waitForQuorumPhase deadline of ${timeout} exceeded`);
    }
  }
}

module.exports = waitForQuorumPhase;
