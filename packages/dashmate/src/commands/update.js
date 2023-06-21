const _ = require('lodash');
const { Flags } = require('@oclif/core');

const ConfigBaseCommand = require('../oclif/command/ConfigBaseCommand');
const printObject = require('../printers/printObject');
const { OUTPUT_FORMATS } = require('../constants');

class UpdateCommand extends ConfigBaseCommand {
  /**
   * @param {Object} args
   * @param {string} format
   * @param {getServiceList} getServiceList
   * @param {docker} docker
   * @param {Config} config
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    {
      format,
    }, getServiceList, docker, config,
  ) {
    const services = getServiceList(config);

    const updated = await Promise.all(
      _.uniqBy(services, 'image')
        .map(async ({ serviceName, image, humanName }) => new Promise((resolve, reject) => {
          docker.pull(image, (err, stream) => {
            let pulled = null;

            stream.on('data', (data) => {
              try {
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
              } catch (e) {
                // eslint-disable-next-line no-empty
              }
            });
            stream.on('error', reject);
            stream.on('end', () => resolve({
              serviceName, humanName, image, pulled,
            }));
          });
        })),
    );

    printObject(updated
      .reduce((acc, { serviceName, humanName, pulled }) => {
        if (format === OUTPUT_FORMATS.PLAIN) {
          return { ...acc, [humanName]: pulled };
        }

        return ({ ...acc, [serviceName]: pulled });
      }, {}), format);
  }
}

UpdateCommand.description = 'Update node software';

UpdateCommand.flags = {
  ...ConfigBaseCommand.flags,
  format: Flags.string({
    description: 'display output format',
    default: OUTPUT_FORMATS.PLAIN,
    options: Object.values(OUTPUT_FORMATS),
  }),
};

module.exports = UpdateCommand;
