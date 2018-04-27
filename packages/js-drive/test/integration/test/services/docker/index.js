/* eslint-disable global-require */
describe('docker', () => {
  require('./Container');
  require('./DockerInstance');
  require('./getAwsEcrAuthorizationToken');
  require('./Image');
  require('./Network');
});
