const createDashCoreInstance = require('./createDashCoreInstance');
const wait = require('../../../util/wait');

/**
 * Start and stop Dashcore instance for mocha tests
 *
 * @return {Promise<DashCoreInstance>}
 */
async function startDashCoreInstance() {
  const instances = await startDashCoreInstance.many(1);

  return instances[0];
}

/**
 * Start and stop a specific number of Dashcore instances for mocha tests
 *
 * @return {Promise<DashCoreInstance[]>}
 */
startDashCoreInstance.many = async function many(number) {
  if (number < 1) {
    throw new Error('Invalid number of instances');
  }

  const instances = [];

  for (let i = 0; i < number; i++) {
    const instance = await createDashCoreInstance();
    await instance.start();

    // Workaround for develop branch
    // We should generate genesis block before we connect instances
    if (i === 0 && number > 1) {
      await instance.getApi().generate(1);
    }

    if (instances.length > 0) {
      await instances[i - 1].connect(instance);
    }
    instances.push(instance);
  }

  // Wait until generate block will be propagated
  if (number > 1) {
    await wait(2000);
  }

  return instances;
};

module.exports = startDashCoreInstance;
