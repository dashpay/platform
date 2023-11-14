import yaml from 'js-yaml';
import fs from 'fs';
import path from 'path';
import { DASHMATE_HELPER_DOCKER_IMAGE, PACKAGE_ROOT_DIR } from '../constants.js';

/**
 * @param {generateEnvs} generateEnvs
 * @param {getConfigProfiles} getConfigProfiles
 * @return {getServiceList}
 */
export default function getServiceListFactory(generateEnvs, getConfigProfiles) {
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

    const profiles = getConfigProfiles(config);

    return Object
      .entries(composeFile.services)
      .map(([serviceName, { image: serviceImage, labels, profiles: serviceProfiles }]) => {
        const title = labels?.['org.dashmate.service.title'];

        if (!title) {
          throw new Error(`Label for dashmate service ${serviceName} is not defined`);
        }

        // Use hardcoded version for dashmate helper
        // Or parse image env variable name and extract version from the env
        const image = serviceName === 'dashmate_helper'
          ? DASHMATE_HELPER_DOCKER_IMAGE : envs[serviceImage.match(/([A-Z_]+)/)[0]];

        return ({
          name: serviceName,
          title,
          image,
          profiles: serviceProfiles ?? [],
        });
      })
      .filter((service) => service.profiles.length === 0
        || service.profiles.some((serviceProfile) => profiles.includes(serviceProfile)));
  }

  return getServiceList;
}
