/* eslint-disable global-require */
describe('SVContract', () => {
  require('./SVContractMongoDbRepository');
  require('./updateSVContractFactory');
  require('./revertSVContractsForStateTransitionFactory');
  require('./fetchDPContractFactory');
});
