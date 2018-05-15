const createDashCoreInstance = require('./createDashCoreInstance');

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
    if (instances.length > 0) {
      await instances[i - 1].connect(instance);
    }
    instances.push(instance);
  }

  return instances;
};

module.exports = startDashCoreInstance;
