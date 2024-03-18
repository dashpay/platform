import fs from 'fs';
import os from 'os';
import { resolve, join } from 'path';
import HomeDirDoesNotExistError from './errors/HomeDirDoesNotExistError.js';
import HomeDirIsNotWritableError from './errors/HomeDirIsNotWritableError.js';
import CouldNotCreateHomeDirError from './errors/CouldNotCreateHomeDirError.js';

export default class HomeDir {
  /**
   * @type {string}
   */
  #path;

  /**
   *
   * @param {string} path - Use the current home dir if not present
   * @param {boolean} [skipValidation=false] - Do not validate path
   */
  constructor(path, skipValidation = false) {
    if (!skipValidation) {
      if (!fs.existsSync(path)) {
        throw new HomeDirDoesNotExistError(path);
      }

      try {
        // eslint-disable-next-line no-bitwise
        fs.accessSync(path, fs.constants.R_OK | fs.constants.W_OK);
      } catch (e) {
        throw new HomeDirIsNotWritableError(path);
      }
    }

    this.#path = path;
  }

  /**
   * Get home dir path
   *
   * @returns {string}
   */
  getPath() {
    return this.#path;
  }

  /**
   * Join paths relative to home dir
   *
   * @param {string} paths
   * @returns {string}
   */
  joinPath(...paths) {
    return join(this.#path, ...paths);
  }

  /**
   * Change home dir path
   * Should be used carefully. Intended to be used for testing only
   *
   * @param {HomeDir} homeDir
   */
  change(homeDir) {
    this.#path = homeDir.getPath();
  }

  /**
   * Remove home dir from file system
   */
  remove() {
    fs.rmSync(this.#path, { recursive: true, force: true });
  }

  /**
   * Create home dir
   *
   * @param {string} [path]
   */
  static createWithPathOrDefault(path) {
    const pathOrDefault = path ?? resolve(os.homedir(), '.dashmate');

    if (!fs.existsSync(pathOrDefault)) {
      try {
        fs.mkdirSync(pathOrDefault);
      } catch (e) {
        throw new CouldNotCreateHomeDirError(pathOrDefault, e);
      }
    }

    return new HomeDir(pathOrDefault);
  }

  /**
   * Create a temporary home dir
   *
   * @returns {HomeDir}
   */
  static createTemp() {
    return new HomeDir(fs.mkdtempSync(join(os.tmpdir(), 'dashmate-')));
  }
}
