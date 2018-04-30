/* eslint-disable global-require */
xdescribe('E2E Tests', () => {
  require('./replication');
  require('./blockchainReorganization');
  require('./sync');
});
