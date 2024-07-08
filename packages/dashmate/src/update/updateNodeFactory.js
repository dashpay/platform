import lodash from 'lodash';

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
        .map(async ({ name, image, title }) => new Promise((resolve) => {
          docker.pull(image, (err, stream) => {
            if (err) {
              if (process.env.DEBUG) {
                // eslint-disable-next-line no-console
                console.error(`Failed to update ${name} service, image ${image}, error: ${err}`);
              }

              resolve({
                name, title, image, updated: 'error',
              });
            } else {
              let updated = null;

              stream.on('data', (data) => {
                // parse all stdout and gather Status message
                const [status] = data
                  .toString()
                  .trim()
                  .split('\r\n')
                  .map((str) => JSON.parse(str))
                  .filter((obj) => obj?.status?.startsWith('Status: '));

                if (status) {
                  if (status.status.includes('Image is up to date for')) {
                    updated = 'up to date';
                  } else if (status.status.includes('Downloaded newer image for')) {
                    updated = 'updated';
                  }
                }
              });
              stream.on('error', () => {
                if (process.env.DEBUG) {
                  // eslint-disable-next-line no-console
                  console.error(`Failed to update ${name} service, image ${image}, error: ${err}`);
                }

                resolve({
                  name, title, image, updated: 'error',
                });
              });
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
