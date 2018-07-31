/* eslint-disable global-require */
describe('Sync', () => {
  require('./initialSync');
  require('./syncInterruption');
  require('./blockchainReorganization');
  require('./throwDashCoreIsNotRunningError');
});
