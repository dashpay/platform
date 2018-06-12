const createIPFSInstance = require('./createIPFSInstance');

/**
 * Start IPFS instance
 *
 * @returns {Promise<IPFSInstance>}
 */
async function startIPFSInstance() {
  const ipfsAPIs = await startIPFSInstance.many(1);

  return ipfsAPIs[0];
}

/**
 * Start specific number of IPFS instance
 *
 * @param {number} number
 * @returns {Promise<IPFSInstance[]>}
 */
startIPFSInstance.many = async function many(number) {
  if (number < 1) {
    throw new Error('Invalid number of instances');
  }

  const instances = [];

  for (let i = 0; i < number; i++) {
    const instance = await createIPFSInstance();
    await instance.start();
    if (instances.length > 0) {
      await instances[i - 1].connect(instance);
    }
    instances.push(instance);
  }

  return instances;
};

module.exports = startIPFSInstance;
