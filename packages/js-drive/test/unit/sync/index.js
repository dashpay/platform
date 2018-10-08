/* eslint-disable global-require */
describe('Sync', () => {
  require('./info');
  require('./state');
  require('./cleanDashDriveFactory');
  require('./isDashCoreRunningFactory');
  require('./isSynced');
  require('./getCheckSyncHttpMiddleware');
});
