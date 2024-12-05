import fs from 'fs';
import path from 'path';
import dots from 'dot';
import os from 'os';
import { TEMPLATES_DIR } from '../../../constants.js';
import wait from '../../../util/wait.js';

export default class VerificationServer {
  /**
   * @param {string} verification url
   */
  #validationUrl;

  /**
   *
   * @param {Docker} docker
   * @param {dockerPull} dockerPull
   * @param {StartedContainers} startedContainers
   * @param {HomeDir} homeDir
   */
  constructor(docker, dockerPull, startedContainers, homeDir) {
    this.docker = docker;
    this.dockerPull = dockerPull;
    this.startedContainers = startedContainers;
    this.homeDir = homeDir;
    this.container = null;
    this.configPath = null;
    this.config = null;
  }

  /**
   * Set up verification server
   *
   * @param {Config} config
   * @param {string} validationUrl
   * @param {string[]} validationContent
   * @return {Promise<void>}
   */
  async setup(config, validationUrl, validationContent) {
    if (this.config) {
      throw new Error('Server is already setup');
    }

    this.#validationUrl = validationUrl;

    this.config = config;

    dots.templateSettings.strip = false;

    // Set up Gateway config
    const configSubPath = path.join('platform', 'gateway');
    const templatePath = path.join(TEMPLATES_DIR, configSubPath, '_zerossl_validation.yaml.dot');
    const templateString = fs.readFileSync(templatePath, 'utf-8');
    const template = dots.template(templateString);

    const route = validationUrl.replace(`http://${config.get('externalIp')}`, '');
    const body = validationContent.join('\\n');

    const gatewayConfig = template({ route, body });

    const configDir = this.homeDir.joinPath(config.getName(), 'platform', 'gateway');
    const configName = path.basename(templatePath, '.dot');

    this.configPath = path.join(configDir, configName);

    if (!fs.existsSync(configDir)) {
      fs.mkdirSync(configDir);
    }
    fs.rmSync(this.configPath, { force: true });
    fs.writeFileSync(this.configPath, gatewayConfig, 'utf8');
  }

  /**
   * Start verification server
   *
   * @return {Promise<boolean>} - False if already started
   */
  async start() {
    if (!this.config) {
      throw new Error('Setup server first');
    }

    if (this.container) {
      return false;
    }

    const image = this.config.get('platform.gateway.docker.image');

    const name = 'dashmate-zerossl-validation';

    const { uid, gid } = os.userInfo();

    const opts = {
      name,
      Image: image,
      Tty: false,
      Env: [`ENVOY_UID=${uid}`, `ENVOY_GID=${gid}`],
      ExposedPorts: { '80/tcp': {} },
      HostConfig: {
        AutoRemove: true,
        Binds: [`${this.configPath}:/etc/envoy/envoy.yaml:ro`],
        PortBindings: { '80/tcp': [{ HostPort: '80' }] },
      },
    };

    await this.dockerPull(image);

    let retries = 0;
    const MAX_RETRIES = 3;
    while (!this.container && retries <= MAX_RETRIES) {
      try {
        this.container = await this.docker.createContainer(opts);
      } catch (e) {
        // Throw any other error except container name conflict
        if (e.statusCode !== 409) {
          throw e;
        }

        // Container name is already in use

        // Remove container
        const danglingContainer = await this.docker.getContainer(name);
        await danglingContainer.remove({ force: true });

        try {
          await danglingContainer.wait();
        } catch (waitError) {
          // Throw any other error except container not found
          if (waitError.statusCode !== 404) {
            throw waitError;
          }

          // Skip error if container is already removed
        }
      }

      retries++;
    }

    this.startedContainers.addContainer(opts.name);

    await this.container.start();

    return true;
  }

  /**
   * Stop verification server
   *
   * @return {Promise<void>}
   */
  async stop() {
    if (!this.container) {
      return;
    }

    await this.container.stop({ t: 3 });

    try {
      await this.container.wait();
    } catch (e) {
      // Skip error if container is already removed
      if (e.statusCode !== 404) {
        throw e;
      }
    }

    this.container = null;
  }

  async waitForServerIsResponding() {
    const MAX_WAIT_TIME = 10000; // Maximum wait time in milliseconds
    const INTERVAL = 500; // Interval to check in milliseconds
    const FETCH_TIMEOUT = 2000; // Timeout for each fetch in ms
    const startTime = Date.now();

    while (Date.now() - startTime < MAX_WAIT_TIME) {
      try {
        const response = await fetch(
          this.#validationUrl,
          { signal: AbortSignal.timeout(FETCH_TIMEOUT) },
        );
        if (response.ok) {
          return true;
        }
      } catch (e) {
        // Ignore errors and continue retrying
      }

      await wait(INTERVAL);
    }

    return false;
  }

  /**
   * Destroy verification server files
   *
   * @return {Promise<void>}
   */
  async destroy() {
    if (!this.config) {
      throw new Error('Setup server first');
    }

    fs.rmSync(this.configPath, { force: true });

    this.config = null;
  }
}
