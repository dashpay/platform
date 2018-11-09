/* eslint-disable global-require */
describe('DapContract', () => {
  require('./DapContractMongoDbRepository');
  require('./updateDapContractFactory');
  require('./revertDapContractsForStateTransitionFactory');
  require('./fetchDapContractFactory');
});
