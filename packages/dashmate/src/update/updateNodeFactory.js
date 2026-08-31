import lodash from 'lodash';
import findPullStreamError from '../docker/findPullStreamError.js';
import sanitizeRemoteText from '../util/sanitizeRemoteText.js';

/**
 * @param {getServiceList} getServiceList
 * @param {docker} docker
 * @return {updateNode}
 */
export default function updateNodeFactory(getServiceList, docker) {
  /**
   * Pulls all recent images by given config
   * @typedef {updateNode}
   *
   * @param {Config} config
   *
   * @return {object[]}
   */
  async function updateNode(config) {
    const services = getServiceList(config);

    return Promise.all(
      lodash.uniqBy(services, 'image')
        .map(async ({
          name, title, image, isBuiltLocally,
        }) => {
          // An image built from sources on this host has nothing to pull
          if (isBuiltLocally) {
            return {
              name, title, image, updated: 'built locally',
            };
          }

          return new Promise((resolve) => {
            docker.pull(image, (err, stream) => {
              if (err) {
                // A registry response body can reach the operator's terminal
                // through this message, so it is made safe to print
                resolve({
                  name, title, image, updated: 'error', error: sanitizeRemoteText(err.message),
                });

                return;
              }

              // followProgress owns the stream: it joins messages split across
              // chunks, splits them the way Docker writes them and reports
              // transport failures. A failed pull arrives as a regular message
              docker.modem.followProgress(stream, (streamError, output) => {
                const error = sanitizeRemoteText(streamError?.message)
                  ?? findPullStreamError(output);

                if (error) {
                  resolve({
                    name, title, image, updated: 'error', error,
                  });

                  return;
                }

                const status = output
                  .find((message) => message?.status?.startsWith('Status: '))
                  ?.status;

                if (status?.includes('Image is up to date for')) {
                  resolve({
                    name, title, image, updated: 'up to date',
                  });

                  return;
                }

                if (status?.includes('Downloaded newer image for')) {
                  resolve({
                    name, title, image, updated: 'updated',
                  });

                  return;
                }

                resolve({
                  name,
                  title,
                  image,
                  updated: 'error',
                  error: 'Docker did not report the pull result',
                });
              });
            });
          });
        }),
    );
  }

  return updateNode;
}
