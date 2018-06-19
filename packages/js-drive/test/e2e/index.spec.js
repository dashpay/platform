/* eslint-disable global-require */
describe('E2E Tests', () => {
  require('./replication');
  require('./blockchainReorganization');
  require('./sync');
});
