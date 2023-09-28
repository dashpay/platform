const fs = require('fs');
const path = require('path');
const dots = require('dot');
const os = require('os');
const { TEMPLATES_DIR } = require('../../../constants');

class VerificationServer {
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

    this.config = config;

    dots.templateSettings.strip = false;

    // Set up Envoy config
    const configSubPath = path.join('platform', 'dapi', 'envoy');
    const templatePath = path.join(TEMPLATES_DIR, configSubPath, '_zerossl_validation.yaml.dot');
    const templateString = fs.readFileSync(templatePath, 'utf-8');
    const template = dots.template(templateString);

    const route = validationUrl.replace(`http://${config.get('externalIp')}`, '');
    const body = validationContent.join('\\n');

    const envoyConfig = template({ route, body });

    const configDir = this.homeDir.joinPath(config.getName(), 'platform', 'dapi', 'envoy');
    const configName = path.basename(templatePath, '.dot');

    this.configPath = path.join(configDir, configName);

    if (!fs.existsSync(configDir)) {
      fs.mkdirSync(configDir);
    }
    fs.rmSync(this.configPath, { force: true });
    fs.writeFileSync(this.configPath, envoyConfig, 'utf8');
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

    const image = this.config.get('platform.dapi.envoy.docker.image');

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

    try {
      this.container = await this.docker.createContainer(opts);
    } catch (e) {
      if (e.statusCode === 409) {
        // Remove container
        const danglingContainer = await this.docker.getContainer(name);

        await danglingContainer.remove({ force: true });

        try {
          await danglingContainer.wait();
        } catch (waitError) {
          // Skip error if container is already removed
          if (e.statusCode !== 404) {
            throw e;
          }
        }

        // Try to create a container one more type
        this.container = await this.docker.createContainer(opts);
      }

      throw e;
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

module.exports = VerificationServer;
