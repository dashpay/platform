/* eslint-disable global-require */
describe('Unit', () => {
  require('./api');
  require('./blockchain');
  require('./mongoDb');
  require('./stateView');
  require('./storage');
  require('./sync');
  require('./util');
});
