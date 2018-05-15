const startHelperWithMochaHooksFactory = require('./startHelperWithMochaHooksFactory');
const startMongoDbInstance = require('../mongoDb/startMongoDbInstance');

module.exports = startHelperWithMochaHooksFactory(startMongoDbInstance);
