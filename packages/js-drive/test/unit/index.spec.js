/* eslint-disable global-require */
describe('Unit', () => {
  require('./api');
  require('./blockchain');
  require('./stateView');
  require('./storage');
  require('./sync');
  require('./test/util');
  require('./util');
});
