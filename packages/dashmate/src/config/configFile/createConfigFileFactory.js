import {getShortHash} from "../../util/getShortHash.js";
import {ConfigFile} from "./ConfigFile.js";
import fs from "fs";
import path from "path";
import {PACKAGE_ROOT_DIR} from '../../constants.js'

/**
 * @param {DefaultConfigs} defaultConfigs
 * @param {HomeDir} homeDir
 * @return {createConfigFile}
 */
export function createConfigFileFactory(defaultConfigs, homeDir) {
  /**
   * @typedef {function} createConfigFile
   * @returns {ConfigFile}
   */
  function createConfigFile() {
    const projectId = getShortHash(homeDir.getPath());

    const {version} = JSON.parse(fs.readFileSync(path.join(PACKAGE_ROOT_DIR, 'package.json'), 'utf8'));

    const configFile = new ConfigFile(
      defaultConfigs.getAll(),
      version,
      projectId,
      null,
      null,
    );

    configFile.markAsChanged();
    configFile.getAllConfigs().forEach((config) => config.markAsChanged());

    return configFile;
  }

  return createConfigFile;
}
