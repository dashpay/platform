const ECR = require('aws-sdk/clients/ecr');

/**
 * Get ECR authorization
 *
 * @param {String} region
 * @return {Promise<authorization>}
 */
async function getAwsEcrAuthorizationToken(region) {
  const registry = new ECR({ region });
  return new Promise((resolve, reject) => {
    registry.getAuthorizationToken((error, authorization) => {
      if (error) {
        return reject(error);
      }
      const {
        authorizationToken,
        proxyEndpoint: serveraddress,
      } = authorization.authorizationData[0];
      const creds = Buffer.from(authorizationToken, 'base64').toString();
      const [username, password] = creds.split(':');
      return resolve({ username, password, serveraddress });
    });
  });
}

module.exports = getAwsEcrAuthorizationToken;
