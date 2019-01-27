/* eslint-disable global-require */
describe('Integration', () => {
  require('./blockchain');
  require('./stateView');
  require('./storage');
  require('./sync/index.js');
  require('./sync.js');
});
