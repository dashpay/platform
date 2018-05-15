/* eslint-disable global-require */
describe('Mocha', () => {
  require('./startDashCoreInstance');
  require('./startDashDriveInstance');
  require('./startIPFSInstance');
  require('./startMongoDbInstance');
});
