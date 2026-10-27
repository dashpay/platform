import fs from 'fs';
import path from 'path';
import semver from 'semver';
import lodash from 'lodash';

import { PACKAGE_ROOT_DIR } from '../constants.js';

const { version } = JSON.parse(fs.readFileSync(path.join(PACKAGE_ROOT_DIR, 'package.json'), 'utf8'));

const prereleaseTag = semver.prerelease(version) === null ? '' : `-${semver.prerelease(version)[0]}`;

/**
 * The image line this dashmate build belongs to: the major on a stable release,
 * the major plus the prerelease identifier on a prerelease.
 */
export const dockerImageVersion = `${semver.major(version)}${prereleaseTag}`;

/**
 * Options whose default is derived from the package version rather than written
 * down anywhere.
 *
 * A config stores null for these to mean "use the image line this dashmate build
 * ships". An explicit string is an operator's own choice and is never touched,
 * which is what makes the distinction reliable: it is recorded rather than
 * guessed at from the shape of the tag.
 */
export const DERIVED_DEFAULTS = {
  'platform.drive.abci.docker.image': () => `dashpay/drive:${dockerImageVersion}`,
  'platform.dapi.rsDapi.docker.image': () => `dashpay/rs-dapi:${dockerImageVersion}`,
};

/**
 * Fill in every derived default left unset, leaving explicit values alone.
 *
 * @param {Object} options
 * @return {Object} a copy with derived defaults resolved
 */
export function resolveDerivedDefaults(options) {
  const resolved = lodash.cloneDeep(options);

  for (const [optionPath, derive] of Object.entries(DERIVED_DEFAULTS)) {
    if (lodash.get(resolved, optionPath) === null) {
      lodash.set(resolved, optionPath, derive());
    }
  }

  return resolved;
}
