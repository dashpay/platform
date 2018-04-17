const DashCoreInstance = require('./DashCoreInstance');

/**
 * Start and stop Dashcore instance for mocha tests
 *
 * @return {Promise<DashCoreInstance>}
 */
async function startDashcoreInstance() {
  const instances = await startDashcoreInstance.many(1);

  return instances[0];
}

/**
 * Start and stop a specific number of Dashcore instances for mocha tests
 *
 * @return {Promise<DashCoreInstance[]>}
 */
startDashcoreInstance.many = async function many(number) {
  if (number < 1) {
    throw new Error('Invalid number of instances');
  }

  const instances = [];

  for (let i = 0; i < number; i++) {
    const instance = new DashCoreInstance();
    await instance.start();
    if (instances.length > 0) {
      await instances[i - 1].connect(instance);
    }
    instances.push(instance);
  }

  after(async () => {
    const promises = instances.map(instance => instance.clean());
    await Promise.all(promises);
  });

  return instances;
};

module.exports = startDashcoreInstance;
