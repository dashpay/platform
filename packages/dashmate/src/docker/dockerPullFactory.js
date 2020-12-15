/**
 * @param {Docker} docker
 * @return {dockerPull}
 */
function dockerPullFactory(docker) {
  /**
   * @typedef {dockerPull}
   * @param {string} image
   * @return {Promise<*>}
   */
  function dockerPull(image) {
    return new Promise((resolve, reject) => {
      docker.pull(image, (err, stream) => {
        if (err) {
          reject(err);

          return;
        }

        docker.modem.followProgress(stream, (progressErr, output) => {
          if (progressErr) {
            reject(progressErr);

            return;
          }

          resolve(output);
        });
      });
    });
  }

  return dockerPull;
}

module.exports = dockerPullFactory;
