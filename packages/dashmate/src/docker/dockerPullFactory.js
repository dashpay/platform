import findPullStreamError from './findPullStreamError.js';

/**
 * @param {Docker} docker
 * @return {dockerPull}
 */
export default function dockerPullFactory(docker) {
  /**
   * @typedef {dockerPull}
   * @param {string} image
   * @param {function} [onProgress] - called with every pull stream message
   * @return {Promise<*>}
   */
  function dockerPull(image, onProgress = undefined) {
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

          // followProgress collects stream messages without inspecting them,
          // so a failed pull has to be recognized here
          const streamError = findPullStreamError(output);

          if (streamError) {
            reject(new Error(streamError));

            return;
          }

          resolve(output);
        }, onProgress);
      });
    });
  }

  return dockerPull;
}
