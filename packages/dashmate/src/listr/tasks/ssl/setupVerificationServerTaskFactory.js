const { Listr } = require('listr2');
const path = require('path');
const fs = require('fs');
const dots = require('dot');
const { HOME_DIR_PATH } = require('../../../constants');

/**
 *
 * @param {Docker} docker
 * @param {StartedContainers} startedContainers
 * @return {setupVerificationServerTask}
 */
function setupVerificationServerTaskFactory(docker, startedContainers) {
  /**
   * @typedef setupVerificationServerTask
   * @param {Config} config
   * @return {Promise<Listr>}
   */
  async function setupVerificationServerTask(config) {
    return new Listr([
      {
        title: 'Setup verification server',
        task: async (ctx) => {
          // Set up template
          const templatePath = path.join(__dirname, '..', '..', '..', '..', 'templates', 'platform', 'dapi', 'envoy', 'ssl.yaml.dot');
          const templateString = fs.readFileSync(templatePath, 'utf-8');
          const template = dots.template(templateString);

          const validationResponse = ctx.response.validation.other_methods[config.get('externalIp')];
          const route = validationResponse.file_validation_url_http.replace(`http://${config.get('externalIp')}`, '');
          const body = validationResponse.file_validation_content.join('\\n');

          // set up envoy config
          const envoyConfig = template({ route, body });
          const configDir = path.join(HOME_DIR_PATH, config.getName());
          const configName = templatePath
            .substring(templatePath.lastIndexOf('/') + 1)
            .replace('.dot', '');
          const absoluteFilePath = path.join(configDir, configName);

          fs.rmSync(absoluteFilePath, { force: true });

          fs.writeFileSync(absoluteFilePath, envoyConfig, 'utf8');

          ctx.envoyConfig = absoluteFilePath;

          const opts = {
            name: 'mn-ssl-verification',
            Image: 'envoyproxy/envoy:v1.22-latest',
            Tty: false,
            ExposedPorts: { '80/tcp': {} },
            HostConfig: {
              AutoRemove: true,
              Binds: [`${ctx.envoyConfig}:/etc/envoy/envoy.yaml:ro`],
              PortBindings: { '80/tcp': [{ HostPort: '80' }] },
            },
          };

          ctx.server = await docker.createContainer(opts);

          startedContainers.addContainer(opts.name);
        },
      }]);
  }

  return setupVerificationServerTask;
}

module.exports = setupVerificationServerTaskFactory;
