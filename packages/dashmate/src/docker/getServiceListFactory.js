const yaml = require('js-yaml');
const fs = require('fs');
const path = require('path');
const { DASHMATE_HELPER_DOCKER_IMAGE, PACKAGE_ROOT_DIR } = require('../constants');

/**
 * @param {generateEnvs} generateEnvs
 * @return {getServiceList}
 */
function getServiceListFactory(generateEnvs) {
  const file = fs.readFileSync(path.join(PACKAGE_ROOT_DIR, 'docker-compose.yml'));
  const composeFile = yaml.load(file);
  
  /**
   * Returns list of services and corresponding docker images from the config
   *
   * @typedef {getServiceList}
   * @param {Config} config
   * @return {Object[]}
   */
  function getServiceList(config) {
    const envs = generateEnvs(config);

    return Object
      .entries(composeFile.services)
      .map(([serviceName, { image: serviceImage, labels, profiles }]) => {
        const title = labels && labels['org.dashmate.service.title'];

        if (!title) {
          throw new Error(`Label for dashmate service ${serviceName} is not defined`);
        }

        // Use hardcoded version for dashmate helper
        // Or parse image variable and extract version from the env
        const image = serviceName === 'dashmate_helper'
          ? DASHMATE_HELPER_DOCKER_IMAGE
          : envs[serviceImage.match(new RegExp(/([A-Z])\w+/))[0]];

        return ({
          name: serviceName,
          title,
          image,
          profiles: profiles ?? [],
        });
      });
  }

  return getServiceList;
}

module.exports = getServiceListFactory;
