const fs = require('fs');
const path = require('path');
const dots = require('dot');
const { HOME_DIR_PATH } = require('../../../constants');

class VerificationServer {
  /**
   *
   * @param {Docker} docker
   * @param {StartedContainers} startedContainers
   */
  constructor(docker, startedContainers) {
    this.docker = docker;
    this.startedContainers = startedContainers;
    this.server = null;
    this.configPath = null;
    this.isRunning = false;
  }

  /**
   * Setup verification server
   *
   * @param {Config} config
   * @param {string} route
   * @param {string} body
   * @return {Promise<void>}
   */
  async setup(config, route, body) {
    if (this.server) {
      throw new Error('Server is already setup');
    }

    // Set up template
    const templatePath = path.join(__dirname, '..', '..', '..', 'ssl', 'templates', 'sslValidation.yaml');
    const templateString = fs.readFileSync(templatePath, 'utf8');
    const template = dots.template(templateString);

    // set up envoy config
    const envoyConfig = template({ route, body });
    const configDir = path.join(HOME_DIR_PATH, config.getName());
    const configName = templatePath
      .substring(templatePath.lastIndexOf('/') + 1)
      .replace('.dot', '');
    this.configPath = path.join(configDir, configName);

    fs.rmSync(this.configPath, { force: true });

    fs.writeFileSync(this.configPath, envoyConfig, 'utf8');

    const opts = {
      name: 'mn-ssl-verification',
      Image: 'envoyproxy/envoy:v1.22-latest',
      Tty: false,
      ExposedPorts: { '80/tcp': {} },
      HostConfig: {
        AutoRemove: true,
        Binds: [`${this.configPath}:/etc/envoy/envoy.yaml:ro`],
        PortBindings: { '80/tcp': [{ HostPort: '80' }] },
      },
    };

    this.server = await this.docker.createContainer(opts);

    this.startedContainers.addContainer(opts.name);
  }

  /**
   * Start verification server
   *
   * @return {Promise<void>}
   */
  async start() {
    if (!this.server) {
      throw new Error('Setup server first');
    }

    if (this.isRunning) {
      return;
    }

    await this.server.start();
  }

  /**
   * Stop verification server
   *
   * @return {Promise<void>}
   */
  async stop() {
    if (!this.isRunning) {
      return;
    }

    await this.server.stop();
  }

  /**
   * Destroy verification server files
   *
   * @return {Promise<void>}
   */
  async destroy() {
    if (!this.server) {
      throw new Error('Setup server first');
    }

    fs.rmSync(this.configPath, { force: true });

    this.server = null;
  }
}

module.exports = VerificationServer;
