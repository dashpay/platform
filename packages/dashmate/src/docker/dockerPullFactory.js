import findPullStreamError from './findPullStreamError.js';
import sanitizeRemoteText from '../util/sanitizeRemoteText.js';

/**
 * Make an error from the daemon safe to print
 *
 * The daemon and the registry behind it choose this text, and callers print it
 * to the operator's terminal.
 *
 * @param {*} error
 * @return {Error}
 */
function sanitizeError(error) {
  if (error instanceof Error) {
    // Kept as the same object, so callers still see its code and status
    // eslint-disable-next-line no-param-reassign
    error.message = sanitizeRemoteText(error.message);

    return error;
  }

  return new Error(sanitizeRemoteText(String(error)));
}

/**
 * @param {Docker} docker
 * @return {dockerPull}
 */
export default function dockerPullFactory(docker) {
  /**
   * @typedef {dockerPull}
   * @param {string} image
   * @param {function} [onProgress] - called with every pull stream message, its
   *                                  text made safe to print
   * @return {Promise<*>}
   */
  function dockerPull(image, onProgress = undefined) {
    const onSafeProgress = onProgress && ((message) => onProgress(
      Object.fromEntries(Object.entries(message ?? {}).map(([key, value]) => (
        [key, sanitizeRemoteText(value)]
      ))),
    ));

    return new Promise((resolve, reject) => {
      docker.pull(image, (err, stream) => {
        if (err) {
          reject(sanitizeError(err));

          return;
        }

        docker.modem.followProgress(stream, (progressErr, output) => {
          if (progressErr) {
            reject(sanitizeError(progressErr));

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
        }, onSafeProgress);
      });
    });
  }

  return dockerPull;
}
