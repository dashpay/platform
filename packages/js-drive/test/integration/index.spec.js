/* eslint-disable global-require */
describe('Integration', () => {
  require('./blockchain');
  require('./storage');
  require('./sync/state/repository');
  require('./test/services');
});
