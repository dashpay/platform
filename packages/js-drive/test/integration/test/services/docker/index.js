/* eslint-disable global-require */
describe('Docker', () => {
  require('./Container');
  require('./DockerInstance');
  require('./getAwsEcrAuthorizationToken');
  require('./Image');
  require('./Network');
});
