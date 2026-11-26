import sanitizeRemoteText from '../util/sanitizeRemoteText.js';

/**
 * Find a failure reported inside a Docker pull progress stream
 *
 * Docker answers a pull request with 200 and then reports registry and disk
 * failures as a message in the progress stream, so a completed stream doesn't
 * mean the image was pulled.
 *
 * The registry chooses the text of that message and it ends up on the
 * operator's terminal, so it is made safe to print before it is passed on.
 *
 * @param {Object[]} output - messages collected from the pull stream
 * @return {string|undefined} failure reason
 */
export default function findPullStreamError(output) {
  const failure = output.find((message) => message?.error);

  if (!failure) {
    return undefined;
  }

  return sanitizeRemoteText(failure.errorDetail?.message ?? failure.error);
}
