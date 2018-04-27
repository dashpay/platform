const Docker = require('dockerode');

class Image {
  /**
   * Create Docker image
   *
   * @param {String} image
   * @param {Object} authorizationToken
   */
  constructor(image, authorizationToken) {
    this.docker = new Docker();
    this.image = image;
    this.authorizationToken = authorizationToken;
  }

  /**
   * Pull image
   *
   * @return {Promise<void>}
   */
  async pull() {
    return new Promise(async (resolve, reject) => {
      try {
        if (this.authorizationToken) {
          const stream = await this.docker.pull(this.image, {
            authconfig: this.authorizationToken,
          });
          return this.docker.modem.followProgress(stream, resolve);
        }

        const stream = await this.docker.pull(this.image);
        return this.docker.modem.followProgress(stream, resolve);
      } catch (error) {
        return reject(error);
      }
    });
  }
}

module.exports = Image;
