import yaml from 'js-yaml';
import fs from 'fs';
import path from 'path';
import lodash from 'lodash';
import { DASHMATE_HELPER_DOCKER_IMAGE, PACKAGE_ROOT_DIR } from '../constants.js';

/**
 * @param {generateEnvs} generateEnvs
 * @param {getConfigProfiles} getConfigProfiles
 * @return {getServiceList}
 */
export default function getServiceListFactory(generateEnvs, getConfigProfiles) {
  /**
   * Returns list of services and corresponding docker images from the config
   *
   * @typedef {getServiceList}
   * @param {Config} config
   * @return {Object[]}
   */
  function getServiceList(config) {
    const envs = generateEnvs(config);

    const composeFiles = envs.COMPOSE_FILE.split(':').map((filenameOrPath) => {
      if (filenameOrPath.startsWith('docker-compose')) {
        const file = fs.readFileSync(path.join(PACKAGE_ROOT_DIR, filenameOrPath));
        return yaml.load(file);
      }

      return null;
    })
      .filter((e) => !!e);

    const profiles = getConfigProfiles(config);

    const composeFile = composeFiles
      // reduce multiple docker compose file into single
      .reduce((composeFilesAcc, currentValue) => lodash.merge(composeFilesAcc, currentValue), {});

    const services = Object.entries(composeFile.services)
      // map to array of services and populate with data
      .map((composeFileServiceEntry) => {
        const [serviceName,
          { image: serviceImage, labels, profiles: serviceProfiles }] = composeFileServiceEntry;

        const title = labels?.['org.dashmate.service.title'];

        if (!title) {
          throw new Error(`Label for dashmate service ${serviceName} is not defined`);
        }

        // Use hardcoded version for dashmate helper
        // Or parse image env variable name and extract version from the env
        const serviceImageEnv = serviceImage.match(/([A-Z_]+)/);

        // eslint-disable-next-line no-nested-ternary
        const image = serviceName === 'dashmate_helper'
          ? DASHMATE_HELPER_DOCKER_IMAGE : serviceImageEnv?.length
            ? envs[serviceImageEnv[0]] : serviceImage;

        return ({
          name: serviceName,
          title,
          image,
          profiles: serviceProfiles ?? [],
        });
      });

    return services.filter((service) => service.profiles.length === 0
      || service.profiles.some((serviceProfile) => profiles.includes(serviceProfile)));
  }

  return getServiceList;
}
