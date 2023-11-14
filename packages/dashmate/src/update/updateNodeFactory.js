import * as _ from 'lodash';

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
  function updateNode(config) {
    const services = getServiceList(config);

    return Promise.all(
      _.uniqBy(services, 'image')
        .map(async ({ name, image, title }) => new Promise((resolve, reject) => {
          docker.pull(image, (err, stream) => {
            if (err) {
              reject(err);
            } else {
              let updated = null;

              stream.on('data', (data) => {
                // parse all stdout and gather Status message
                const [status] = data
                  .toString()
                  .trim()
                  .split('\r\n')
                  .map((str) => JSON.parse(str))
                  .filter((obj) => obj.status.startsWith('Status: '));

                if (status?.status.includes('Image is up to date for')) {
                  updated = false;
                } else if (status?.status.includes('Downloaded newer image for')) {
                  updated = true;
                }
              });
              stream.on('error', reject);
              stream.on('end', () => resolve({
                name, title, image, updated,
              }));
            }
          });
        })),
    );
  }

  return updateNode;
}
