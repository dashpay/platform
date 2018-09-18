/* eslint-disable global-require */
describe('Integration', () => {
  require('./api');
  require('./blockchain');
  require('./stateView');
  require('./storage');
  require('./sync');
});
