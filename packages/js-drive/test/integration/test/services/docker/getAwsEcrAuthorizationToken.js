const getAwsEcrAuthorizationToken = require('../../../../../lib/test/services/docker/getAwsEcrAuthorizationToken');

describe('getAwsEcrAuthorizationToken', () => {
  it('should get the authorization', async () => {
    const authorization = await getAwsEcrAuthorizationToken(process.env.AWS_DEFAULT_REGION);
    expect(authorization.username).to.exist();
    expect(authorization.password).to.exist();
    expect(authorization.serveraddress).to.exist();
  });
});
