/* eslint-disable global-require */
describe('Sync', () => {
  require('./cleanDashDriveFactory');
  require('./isDashCoreRunningFactory');
  require('./isSynced');
  require('./getCheckSyncHttpMiddleware');
  require('./state');
});
