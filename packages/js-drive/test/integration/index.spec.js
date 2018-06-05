/* eslint-disable global-require */
describe('Integration', () => {
  require('./blockchain');
  require('./stateView');
  require('./storage');
  require('./sync/state/repository');
  require('./test/services');
});
