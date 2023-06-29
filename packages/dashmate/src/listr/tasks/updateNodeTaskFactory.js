const _ = require('lodash');

/**
 * @param {getServiceList} getServiceList
 * @param {docker} docker
 * @return {updateNodeTask}
 */
function updateNodeTaskFactory(getServiceList, docker) {
  /**
   * Pulls all recent images by given config
   * @typedef {updateNodeTask}
   *
   * @param {Config} config
   *
   * @return {object[]}
   */
  function updateNodeTask(config) {
    const services = getServiceList(config);

    return Promise.all(
      _.uniqBy(services, 'image')
        .map(async ({ serviceName, image, title }) => new Promise((resolve, reject) => {
          docker.pull(image, (err, stream) => {
            if (err) {
              reject(err);
            } else {
              let pulled = null;

              stream.on('data', (data) => {
                // parse all stdout and gather Status message
                const [status] = data
                  .toString()
                  .trim()
                  .split('\r\n')
                  .map((str) => JSON.parse(str))
                  .filter((obj) => obj.status.startsWith('Status: '));

                if (status?.status.includes('Image is up to date for')) {
                  pulled = false;
                } else if (status?.status.includes('Downloaded newer image for')) {
                  pulled = true;
                }
              });
              stream.on('error', reject);
              stream.on('end', () => resolve({
                serviceName, title, image, pulled,
              }));
            }
          });
        })),
    );
  }

  return updateNodeTask;
}

module.exports = updateNodeTaskFactory;
