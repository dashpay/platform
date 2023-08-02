const yaml = require('js-yaml');
const fs = require('fs');
const path = require('path');
const { DASHMATE_HELPER_DOCKER_IMAGE, PACKAGE_ROOT_DIR } = require('../constants');

/**
 * @param {generateEnvs} generateEnvs
 * @return {getServiceList}
 */
function getServiceListFactory(generateEnvs) {
  /**
   * Returns list of services and corresponding docker images from the config
   *
   * @typedef {getServiceList}
   * @param {Config} config
   * @return {Object[]}
   */
  function getServiceList(config) {
    const file = yaml.load(fs.readFileSync(path.join(PACKAGE_ROOT_DIR, 'docker-compose.yml')));

    const envs = generateEnvs(config);

    return Object
      .entries(file.services)
      .map(([key, { image, labels, profiles }]) => ({
        name: key,
        title: labels['org.dashmate.service.title'],
        image: key === 'dashmate_helper' ? DASHMATE_HELPER_DOCKER_IMAGE
          : envs[image.match(new RegExp(/([A-Z])\w+/))[0]],
        profiles: profiles ?? [],
      }));
  }

  return getServiceList;
}

module.exports = getServiceListFactory;
