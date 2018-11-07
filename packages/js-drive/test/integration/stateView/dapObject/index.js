/* eslint-disable global-require */
describe('DapObject', () => {
  require('./createDapObjectMongoDbRepositoryFactory');
  require('./DapObjectMongoDbRepository');
  require('./fetchDapObjectsFactory');
  require('./updateDapObjectFactory');
  require('./revertDapObjectsForStateTransitionFactory');
});
