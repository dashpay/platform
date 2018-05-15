const createMongoDbInstance = require('./createMongoDbInstance');

/**
 * Start and stop MongoDb instance for mocha tests
 *
 * @return {Promise<MongoDbInstance>}
 */
async function startMongoDbInstance() {
  const instances = await startMongoDbInstance.many(1);

  return instances[0];
}

/**
 * Start and stop a specific number of MongoDb instances for mocha tests
 *
 * @return {Promise<MongoDbInstance[]>}
 */
startMongoDbInstance.many = async function many(number) {
  if (number < 1) {
    throw new Error('Invalid number of instances');
  }

  const instances = [];

  for (let i = 0; i < number; i++) {
    const instance = await createMongoDbInstance();
    await instance.start();
    instances.push(instance);
  }

  return instances;
};

module.exports = startMongoDbInstance;
