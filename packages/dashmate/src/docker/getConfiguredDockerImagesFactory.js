import fs from 'node:fs';
import path from 'node:path';
import yaml from 'js-yaml';
import { PACKAGE_ROOT_DIR } from '../constants.js';

/**
 * Collect protection references, not cleanup candidates. Include disabled
 * profiles, local builds and generated Compose overrides in every config.
 *
 * @param {ConfigFile} configFile
 * @param {generateEnvs} generateEnvs
 * @returns {getConfiguredDockerImages}
 */
export default function getConfiguredDockerImagesFactory(configFile, generateEnvs) {
  /**
   * @typedef {function} getConfiguredDockerImages
   * @returns {Set<string>}
   */
  function getConfiguredDockerImages() {
    const references = new Set();
    const shippedFiles = fs.readdirSync(PACKAGE_ROOT_DIR)
      .filter((name) => /^docker-compose(?:\..+)?\.yml$/.test(name))
      .map((name) => path.join(PACKAGE_ROOT_DIR, name));

    for (const config of configFile.getAllConfigs()) {
      const envs = generateEnvs(config);
      Object.entries(envs).forEach(([key, value]) => {
        if (key.endsWith('_DOCKER_IMAGE') && value) {
          references.add(value);
        }
      });

      const files = new Set([
        ...shippedFiles,
        ...envs.COMPOSE_FILE.split(':').map((file) => path.resolve(PACKAGE_ROOT_DIR, file)),
      ]);

      for (const file of files) {
        let contents;
        try {
          contents = fs.readFileSync(file, 'utf8');
        } catch (error) {
          // A dormant config may not have rendered its generated file yet.
          if (error.code === 'ENOENT' && path.basename(file) === 'dynamic-compose.yml') {
            continue;
          }
          throw error;
        }

        const compose = yaml.load(contents);
        for (const service of Object.values(compose?.services ?? {})) {
          const reference = service?.image;
          if (reference === undefined) {
            continue;
          }
          if (typeof reference !== 'string') {
            throw new Error('Cannot safely resolve a configured Docker image');
          }

          // The shipped files use literals or a single required environment
          // variable. Do not guess at more complex/custom Compose interpolation.
          const variable = reference.match(/^\$\{([A-Z_]+)(?::\?[^}]*)?\}$/);
          if (variable && Object.hasOwn(envs, variable[1])) {
            if (envs[variable[1]]) {
              references.add(envs[variable[1]]);
            }
          } else if (reference.includes('$')) {
            throw new Error('Cannot safely resolve a configured Docker image');
          } else {
            references.add(reference);
          }
        }
      }
    }

    return references;
  }

  return getConfiguredDockerImages;
}
