const { PrivateKey } = require('@dashevo/dashcore-lib');
const wait = require('../util/wait');

/**
 * Wait Core to set quorum
 *
 * @typedef {enableCoreQuorums}
 * @param {RpcClient} rpcClient
 * @param {string} network
 * @return {Promise<void>}
 */
async function enableCoreQuorums(rpcClient, network) {
  const privateKey = new PrivateKey();
  const address = privateKey.toAddress(network).toString();

  let hasQuorums = false;

  do {
    const { result: quorums } = await rpcClient.quorum('list');

    if (quorums) {
      for (const quorum of Object.values(quorums)) {
        if (quorum.length > 1) {
          hasQuorums = true;
        }
      }
    }

    if (!hasQuorums) {
      await rpcClient.generateToAddress(1, address, 10000000);
      await wait(5000);
    }
  } while (!hasQuorums);
}

module.exports = enableCoreQuorums;
