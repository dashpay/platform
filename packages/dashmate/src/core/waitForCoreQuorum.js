const wait = require('../util/wait');

/**
 * Wait Core to set quorum
 *
 * @typedef {waitForCoreQuorum}
 * @param {RpcClient} rpcClient
 * @return {Promise<void>}
 */
async function waitForCoreQuorum(rpcClient) {
  let hasQuorums = false;

  do {
    const { result: quorums } = await rpcClient.quorum('list');

    if (quorums) {
      for (const quorum of Object.values(quorums)) {
        if (quorum.length > 0) {
          hasQuorums = true;
        }
      }
    }

    if (!hasQuorums) {
      await wait(10000);
    }
  } while (!hasQuorums);
}

module.exports = waitForCoreQuorum;
